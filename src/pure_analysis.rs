//! src/pure_analysis.rs — WS-4 Phase 2 pure-Rust image-feature analyzer (Lane A).
//!
//! Pure-Rust mirror of the OpenCV `image_analysis.rs` extraction, built on the
//! `image` + `imageproc` crates. It produces the engine's image-free mirror
//! structs (`engine::GlobalFeatures` / `engine::ScanBarFeatures`) DIRECTLY and
//! implements `engine::FeatureSource`, so the engine core is byte-unchanged and
//! the OpenCV adapter is no longer on the default build path.
//!
//! Boundary: this module names NO OpenCV type and NO `image_analysis` type. It
//! reads pixels via the pure-Rust `image` crate and computes HSV stats / Canny
//! edge density / Laplacian texture variance / an 8-bin hue histogram / a
//! connected-component shape-complexity proxy. Feature-fidelity deltas vs OpenCV
//! are documented in design-s11 §2 (the owner's ear is the parity gate).
//!
//! IMPLEMENTER NOTE (dep-version reconciliation — conflicts with design §5/§ appendix):
//! The design names `imageproc = "0.25"` "[VERIFY 0.25 + image 0.24 compat]". On
//! verification (2026-06-13) that combination does NOT hold: `imageproc 0.24+`
//! depends on `image 0.25`, which would pull a SECOND, incompatible major of the
//! `image` crate into the tree (the project pins `image 0.24`, and `engine`/this
//! module pass `image::Rgb<u8>` across the boundary — two `image` majors do not
//! interop). The version that pins `image 0.24` is `imageproc = "0.23"`, so Lane A
//! uses 0.23. The only API delta this forces: `imageproc 0.23` has no
//! `filter::laplacian_filter` (added later) — so the Laplacian is hand-rolled in
//! f64 here (see [`laplacian_var_pure`]), which is actually CLOSER to OpenCV's
//! `laplacian(CV_64F)` (no i16 clamp) than imageproc's filter would be. Reported
//! to the lead.

use image::{GenericImageView, GrayImage, ImageBuffer, Luma, Rgb, RgbImage};

use crate::engine::{FeatureSource, GlobalFeatures, ScanBarFeatures};

/// Error type for the pure analyzer (empty image, decode failure, zero bars).
/// Maps to the same failure cases `image_analysis`'s `anyhow!` guards cover.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    /// An empty (zero-pixel) image reached an analyzer entry point.
    #[error("empty image passed to {0}")]
    EmptyImage(&'static str),
    /// An image failed to decode/load from disk.
    #[error("image decode/load failed: {0}")]
    Decode(String),
    /// A scan with `num_bars == 0` was requested.
    #[error("num_bars must be > 0")]
    ZeroBars,
}

/// A loaded image in the pure-Rust path. Owns an 8-bit RGB buffer; no OpenCV `Mat`.
///
/// theory: the analyzer needs random pixel access + cheap rectangular sub-views
/// for scan strips; `image::RgbImage` gives both (`crop_imm` is a zero-copy view).
pub struct PureImage {
    /// width/height accessible via `image::GenericImageView`.
    inner: RgbImage,
}

impl PureImage {
    /// Wrap an in-memory `RgbImage` (the test/headless construction path).
    pub fn from_rgb(inner: RgbImage) -> Self {
        PureImage { inner }
    }

    /// Borrow the underlying RGB buffer.
    pub fn as_rgb(&self) -> &RgbImage {
        &self.inner
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        self.inner.height()
    }
}

/// Image source for the pure path. Mirrors the subset of `image_source::ImageSource`
/// that does not require OpenCV. Camera/AI-generated are intentionally absent (the
/// `opencv` flag retains camera capture; AI-gen is still a placeholder elsewhere).
pub enum PureImageSource {
    /// A filename relative to `assets/images/`.
    Preselected(String),
    /// An arbitrary filesystem path.
    UserPath(std::path::PathBuf),
}

/// Load an image from a pure source into a `PureImage` (JPEG/PNG via the `image`
/// crate; `jpeg-decoder` is already a transitive dep). Replaces
/// `image_source::load_image_from_source` on the default path.
pub fn load_pure_image(src: &PureImageSource) -> Result<PureImage, AnalysisError> {
    let path = match src {
        PureImageSource::Preselected(name) => {
            std::path::Path::new("assets").join("images").join(name)
        }
        PureImageSource::UserPath(p) => p.clone(),
    };
    let dynimg = image::open(&path)
        .map_err(|e| AnalysisError::Decode(format!("{}: {e}", path.display())))?;
    let rgb = dynimg.to_rgb8();
    if rgb.width() == 0 || rgb.height() == 0 {
        return Err(AnalysisError::EmptyImage("load_pure_image"));
    }
    Ok(PureImage { inner: rgb })
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-pixel color conversion (parity-critical: OpenCV output ranges)
// ─────────────────────────────────────────────────────────────────────────────

/// Per-pixel RGB→HSV in OpenCV's output ranges: H 0..360, S 0..100, V 0..100 (the
/// `image_analysis.rs:94..96` conventions).
///
/// theory: OpenCV's `cvt_color(BGR2HSV)` produces 8-bit H in 0..179, S/V in 0..255,
/// then `image_analysis` rescales: `avg_hue = mean_h * 2`, `avg_saturation =
/// mean_s * 100/255`, `avg_brightness = mean_v * 100/255`. We compute true
/// floating-point HSV directly and emit it already in those final ranges (H in
/// DEGREES 0..360, S/V as 0..100 percent), so the means line up with OpenCV's
/// rescaled means modulo 8-bit rounding (a sub-1% drift — design §2). Pure
/// arithmetic; no dependency.
fn rgb_to_hsv(p: Rgb<u8>) -> (f32, f32, f32) {
    let r = p[0] as f32 / 255.0;
    let g = p[1] as f32 / 255.0;
    let b = p[2] as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // Hue in degrees 0..360 (the standard HSV hue circle; OpenCV's 0..179 is this
    // same circle halved into 8 bits, which `image_analysis` then doubles back).
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        // max == r
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        // max == g
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        // max == b
        60.0 * (((r - g) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    // Saturation and Value as 0..100 percent (HSV "value" == max).
    let sat = if max <= f32::EPSILON {
        0.0
    } else {
        (delta / max) * 100.0
    };
    let val = max * 100.0;

    (hue, sat, val)
}

// ─────────────────────────────────────────────────────────────────────────────
// HSV aggregation (circular mean for hue — design §3.A.4)
// ─────────────────────────────────────────────────────────────────────────────

/// Mean H (0..360), S (0..100), V (0..100) over a pixel iterator.
///
/// theory: hue is a CIRCULAR quantity — averaging raw 0..360 values mishandles the
/// red wrap (0≈360). The DEFAULT uses the circular mean (mean of unit vectors at
/// each pixel's hue angle); a `compat_arithmetic` flag reproduces OpenCV's
/// arithmetic mean of the (degree) hue values for A/B parity (design §2). S and V
/// are ordinary arithmetic means in both modes (they are linear, not circular).
fn hsv_means<I: Iterator<Item = Rgb<u8>>>(pixels: I, compat_arithmetic: bool) -> (f32, f32, f32) {
    let mut n: u64 = 0;
    let mut sum_s = 0.0f64;
    let mut sum_v = 0.0f64;
    // Circular accumulators (mean of cos/sin of the hue angle).
    let mut sum_cos = 0.0f64;
    let mut sum_sin = 0.0f64;
    // Arithmetic-compat accumulator (raw degrees).
    let mut sum_h = 0.0f64;

    for p in pixels {
        let (h, s, v) = rgb_to_hsv(p);
        n += 1;
        sum_s += s as f64;
        sum_v += v as f64;
        sum_h += h as f64;
        let rad = (h as f64).to_radians();
        sum_cos += rad.cos();
        sum_sin += rad.sin();
    }

    if n == 0 {
        return (0.0, 0.0, 0.0);
    }
    let nf = n as f64;
    let avg_s = (sum_s / nf) as f32;
    let avg_v = (sum_v / nf) as f32;

    let avg_h = if compat_arithmetic {
        (sum_h / nf) as f32
    } else {
        // atan2 of the summed unit vectors → mean angle, normalized into 0..360.
        let mut ang = (sum_sin / nf).atan2(sum_cos / nf).to_degrees();
        if ang < 0.0 {
            ang += 360.0;
        }
        ang as f32
    };

    (avg_h, avg_s, avg_v)
}

/// Circular standard deviation of hue, scaled to match OpenCV's `stddev_h/90`
/// `hue_spread` heuristic (`image_analysis.rs:162`). Returns roughly 0..1.
///
/// theory: OpenCV took the arithmetic stddev of the 0..179 H channel and divided by
/// 90 (so a flat-hue image → 0, a maximally spread one → ~1). The circular stddev
/// is `sqrt(-2 ln R)` where R is the mean resultant length of the unit hue vectors;
/// it ranges 0 (all hues equal) upward and is the correct dispersion for a circular
/// quantity. We express it in OpenCV H-channel units (degrees / 2, matching the
/// 0..179 scale OpenCV's stddev was on) before dividing by 90, so the magnitude
/// lines up with the OpenCV heuristic. Near the red wrap it is more correct and
/// will differ from OpenCV (design §2).
fn hue_spread_pure<I: Iterator<Item = Rgb<u8>>>(pixels: I) -> f32 {
    let mut n: u64 = 0;
    let mut sum_cos = 0.0f64;
    let mut sum_sin = 0.0f64;
    for p in pixels {
        let (h, _s, _v) = rgb_to_hsv(p);
        n += 1;
        let rad = (h as f64).to_radians();
        sum_cos += rad.cos();
        sum_sin += rad.sin();
    }
    if n == 0 {
        return 0.0;
    }
    let nf = n as f64;
    // Mean resultant length R ∈ [0,1].
    let r = ((sum_cos / nf).powi(2) + (sum_sin / nf).powi(2)).sqrt();
    let r = r.clamp(1e-12, 1.0);
    // Circular stddev in RADIANS, then → degrees.
    let circ_std_deg = (-2.0 * r.ln()).sqrt().to_degrees();
    // OpenCV's stddev was on the 0..179 (half-degree) H scale, so halve to those
    // units before the /90 the heuristic uses.
    ((circ_std_deg / 2.0) / 90.0) as f32
}

/// Normalized 8-bin hue histogram (sum=1) matching `compute_hue_histogram`
/// (`image_analysis.rs:351`). Carried for fidelity; music-inert.
///
/// theory: OpenCV binned the 0..180 H channel into `bins` equal buckets and
/// normalized by the sum. We bin the per-pixel hue (in degrees 0..360, mapped to
/// the same 0..180 OpenCV H scale via /2) identically and normalize by sum.
fn hue_histogram_pure<I: Iterator<Item = Rgb<u8>>>(pixels: I, bins: usize) -> Vec<f32> {
    if bins == 0 {
        return Vec::new();
    }
    let mut hist = vec![0.0f32; bins];
    let mut total = 0.0f32;
    for p in pixels {
        let (h, _s, _v) = rgb_to_hsv(p);
        // Degrees 0..360 → OpenCV H units 0..180.
        let h_cv = (h / 2.0).clamp(0.0, 179.999);
        // Bucket over the 0..180 range (matches OpenCV's `ranges = [0,180]`).
        let mut idx = ((h_cv / 180.0) * bins as f32) as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        hist[idx] += 1.0;
        total += 1.0;
    }
    if total > 0.0 {
        for v in hist.iter_mut() {
            *v /= total;
        }
    }
    hist
}

// ─────────────────────────────────────────────────────────────────────────────
// Grayscale derivation + edge / texture / shape kernels
// ─────────────────────────────────────────────────────────────────────────────

/// Convert an RGB view to an 8-bit grayscale `GrayImage` using OpenCV's BGR2GRAY
/// luma weights (0.114·B + 0.587·G + 0.299·R), expressed here in RGB terms as the
/// standard Rec.601 luma so the gray channel matches OpenCV's `cvt_color(BGR2GRAY)`.
fn to_gray<V: GenericImageView<Pixel = Rgb<u8>>>(view: &V) -> GrayImage {
    let (w, h) = view.dimensions();
    let mut g = GrayImage::new(w, h);
    for (x, y, px) in view.pixels() {
        let r = px[0] as f32;
        let gg = px[1] as f32;
        let b = px[2] as f32;
        // Rec.601 luma (same coefficients OpenCV uses for {BGR,RGB}2GRAY).
        let luma = (0.299 * r + 0.587 * gg + 0.114 * b)
            .round()
            .clamp(0.0, 255.0) as u8;
        g.put_pixel(x, y, Luma([luma]));
    }
    g
}

/// Canny edge density 0..1 over a grayscale image. Uses `imageproc::edges::canny`
/// with the same 50/150 hysteresis thresholds as OpenCV (`image_analysis.rs:108`),
/// then non-zero / total.
///
/// DELTA vs OpenCV (design §2): imageproc's Canny and OpenCV's Canny share the
/// algorithm (blur → Sobel → non-max suppression → hysteresis) but differ in the
/// default Gaussian kernel and the gradient norm (L1 vs L2), so edge counts differ
/// by a few percent. Music-coarse; A/B on real images is the gate.
fn edge_density_pure(gray: &GrayImage) -> f32 {
    let (w, h) = gray.dimensions();
    let total = (w as f32) * (h as f32);
    if total <= 0.0 {
        return 0.0;
    }
    // canny needs at least a small image; on a tiny strip the output is still valid.
    let edges: GrayImage = imageproc::edges::canny(gray, 50.0, 150.0);
    let nonzero = edges.pixels().filter(|p| p[0] != 0).count() as f32;
    nonzero / total
}

/// Population variance of the Laplacian response (focus/texture). Mirrors
/// `image_analysis.rs:114..119`.
///
/// theory + DELTA: OpenCV used `laplacian(CV_64F, ksize=3)` (the 4-neighbour 3×3
/// kernel `[[0,1,0],[1,-4,1],[0,1,0]]` with BORDER_DEFAULT reflect) then
/// `stddev²` of the f64 response. `imageproc 0.23` has no `laplacian_filter`, and
/// `filter3x3` would CLAMP the response to an integer channel (losing the negative
/// tails and large magnitudes the variance depends on). So we hand-roll the 3×3
/// Laplacian convolution in f64 over the gray image with REFLECT border handling
/// — matching OpenCV's CV_64F path far more faithfully than a clamped integer
/// filter would. We then return the population variance. Differs from OpenCV only
/// by f64-vs-f64 rounding and the exact reflect-vs-clamp at the 1-px border.
fn laplacian_var_pure(gray: &GrayImage) -> f32 {
    let (w, h) = gray.dimensions();
    if w == 0 || h == 0 {
        return 0.0;
    }
    let wi = w as i64;
    let hi = h as i64;
    // Reflect-101-style border (OpenCV BORDER_DEFAULT): mirror without repeating the
    // edge pixel. For a 1-pixel kernel reach, clamp-to-edge and reflect coincide on
    // the interior; we use reflect for the borders. Implemented via an index map.
    let sample = |xx: i64, yy: i64| -> f64 {
        // Reflect index into [0, n-1] without repeating the boundary (101 reflect).
        let reflect = |mut i: i64, n: i64| -> i64 {
            if n == 1 {
                return 0;
            }
            // Period is 2*(n-1); reflect into a triangle wave.
            let period = 2 * (n - 1);
            i = ((i % period) + period) % period;
            if i >= n {
                i = period - i;
            }
            i
        };
        let rx = reflect(xx, wi) as u32;
        let ry = reflect(yy, hi) as u32;
        gray.get_pixel(rx, ry)[0] as f64
    };

    // First pass: compute the Laplacian response and accumulate mean.
    let mut responses: Vec<f64> = Vec::with_capacity((w as usize) * (h as usize));
    let mut sum = 0.0f64;
    for y in 0..hi {
        for x in 0..wi {
            // 4-neighbour Laplacian: up+down+left+right - 4*center.
            let lap = sample(x, y - 1) + sample(x, y + 1) + sample(x - 1, y) + sample(x + 1, y)
                - 4.0 * sample(x, y);
            responses.push(lap);
            sum += lap;
        }
    }
    let n = responses.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mean = sum / n;
    // Population variance (OpenCV's mean_std_dev uses the population stddev, /N).
    let var = responses
        .iter()
        .map(|r| (r - mean) * (r - mean))
        .sum::<f64>()
        / n;
    var as f32
}

/// Connected-component count / 1000 as the `shape_complexity` PROXY for OpenCV's
/// external-contour count (`image_analysis.rs:122..139`).
///
/// theory + DELTA (the LARGEST honest fidelity delta — design §2): OpenCV counts
/// external contours via `find_contours(RETR_EXTERNAL)` after an Otsu threshold;
/// we Otsu-threshold (`imageproc::contrast::otsu_level` + `threshold`) and then
/// count connected components (`region_labelling::connected_components`, 8-conn,
/// background = 0). These are DIFFERENT segmentation algorithms — counts will not
/// match. But `shape_complexity = count/1000` is a self-described crude heuristic
/// feeding a coarse mapping; the owner A/Bs and we tune the /1000 normalization if
/// the ear flags it.
fn shape_complexity_pure(gray: &GrayImage) -> f32 {
    let (w, h) = gray.dimensions();
    if w == 0 || h == 0 {
        return 0.0;
    }
    let level = imageproc::contrast::otsu_level(gray);
    let binary: GrayImage = imageproc::contrast::threshold(gray, level);
    use imageproc::region_labelling::{connected_components, Connectivity};
    // Background colour is Luma([0]) (same u8 pixel type as the input); foreground
    // blobs get distinct labels 1..=N (the OUTPUT pixels are u32 labels).
    let labelled: ImageBuffer<Luma<u32>, Vec<u32>> =
        connected_components(&binary, Connectivity::Eight, Luma([0u8]));
    // The number of components is the maximum label assigned (labels are 1..=count;
    // background pixels are 0).
    let count = labelled.pixels().map(|p| p[0]).max().unwrap_or(0);
    (count as f32) / 1000.0
}

// ─────────────────────────────────────────────────────────────────────────────
// Whole-image + per-section feature assembly
// ─────────────────────────────────────────────────────────────────────────────

/// Whole-image features over the full RGB buffer. Mirrors `analyze_global`
/// (`image_analysis.rs:57`) field-for-field, producing `engine::GlobalFeatures`.
pub fn analyze_global_pure(img: &RgbImage) -> Result<GlobalFeatures, AnalysisError> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err(AnalysisError::EmptyImage("analyze_global_pure"));
    }
    let (avg_hue, avg_saturation, avg_brightness) = hsv_means(img.pixels().copied(), false);
    let hue_spread = hue_spread_pure(img.pixels().copied());

    let gray = to_gray(img);
    let edge_density = edge_density_pure(&gray);
    let texture_laplacian_var = laplacian_var_pure(&gray);
    let shape_complexity = shape_complexity_pure(&gray);

    let aspect_ratio = (w as f32) / (h as f32);

    Ok(GlobalFeatures {
        avg_hue,
        avg_saturation,
        avg_brightness,
        edge_density,
        hue_spread,
        texture_laplacian_var,
        shape_complexity,
        aspect_ratio,
    })
}

/// One scan-bar section's features over a sub-view. Mirrors the per-section work in
/// `scan_image`'s inner loop (`image_analysis.rs:507..521`), producing one
/// `engine::ScanBarFeatures`. `bar_index` is the section's index in the row.
///
/// theory: the OpenCV path routed each section through `analyze_local_basic`, which
/// produced hue/saturation/brightness (HSV means), `edge_sharpness` (Canny edge
/// density), `texture_complexity` (Laplacian var), plus an 8-bin hue histogram.
/// Those map field-for-field onto `ScanBarFeatures` (`edge_sharpness → edge_density`,
/// `texture_complexity → texture_laplacian_var`). We reproduce exactly those.
pub fn analyze_section_pure(
    section: &RgbImage,
    bar_index: usize,
) -> Result<ScanBarFeatures, AnalysisError> {
    let (w, h) = section.dimensions();
    if w == 0 || h == 0 {
        return Err(AnalysisError::EmptyImage("analyze_section_pure"));
    }
    // The caller hands an OWNED section buffer (cropped via `crop_imm(..).to_image()`
    // in `scan_steps`) — `image 0.24`'s `SubImage` does not impl `GenericImageView`,
    // so we take an owned `RgbImage` and iterate it directly for the HSV passes and
    // the gray derivation.
    let (avg_hue, avg_saturation, avg_brightness) = hsv_means(section.pixels().copied(), false);
    let hue_hist = hue_histogram_pure(section.pixels().copied(), 8);
    let gray = to_gray(section);
    let edge_density = edge_density_pure(&gray);
    let texture_laplacian_var = laplacian_var_pure(&gray);

    Ok(ScanBarFeatures {
        bar_index,
        avg_hue,
        avg_saturation,
        avg_brightness,
        edge_density,
        texture_laplacian_var,
        hue_hist,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Scan geometry (parity with image_analysis::scan_image rect math) + the source
// ─────────────────────────────────────────────────────────────────────────────

/// A pixel rectangle (x, y, w, h) — the pure-Rust analogue of OpenCV's
/// `core::Rect`, used to mirror `scan_image`'s rect math exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// Pre-extracted pure-Rust features for one image, ready to serve through
/// `engine::FeatureSource`. Built once from a `PureImage` + the pipeline geometry
/// (instrument count, bar thickness, step count), mirroring the OpenCV
/// `PrecomputedSource` shape so the engine sees an identical feature stream.
#[derive(Debug, Clone)]
pub struct PureAnalysisSource {
    global: GlobalFeatures,
    steps: Vec<Vec<ScanBarFeatures>>,
}

impl PureAnalysisSource {
    /// Extract whole-image + per-step features from `img`. `num_instruments`,
    /// `bar_thickness_frac`, `num_steps`, and `vertical_hint` use the SAME rect
    /// geometry as `image_analysis::scan_image` so the per-step rows line up 1:1
    /// with the OpenCV path (design §3.A geometry parity).
    pub fn extract(
        img: &PureImage,
        num_instruments: usize,
        bar_thickness_frac: f32,
        num_steps: usize,
        vertical_hint: Option<bool>,
    ) -> Result<Self, AnalysisError> {
        let rgb = &img.inner;
        let (width, height) = rgb.dimensions();
        if width == 0 || height == 0 {
            return Err(AnalysisError::EmptyImage("PureAnalysisSource::extract"));
        }
        if num_instruments == 0 {
            return Err(AnalysisError::ZeroBars);
        }

        let global = analyze_global_pure(rgb)?;
        let steps = Self::scan_steps(
            rgb,
            num_instruments,
            bar_thickness_frac,
            num_steps,
            vertical_hint,
        )?;

        Ok(PureAnalysisSource { global, steps })
    }

    /// Mirror of `image_analysis::scan_image` rect math (lines 421..528), producing
    /// `Vec<Vec<ScanBarFeatures>>` over the SAME sub-rectangles. Geometry is
    /// computed in i64 to match the OpenCV i32 arithmetic (floor division, the
    /// last-section-absorbs-remainder rule, the `.max(1)` clamps, the round-to-int
    /// step travel) and then clipped to the image bounds for the pure crop.
    fn scan_steps(
        rgb: &RgbImage,
        num_bars: usize,
        bar_thickness_frac: f32,
        num_steps: usize,
        vertical_hint: Option<bool>,
    ) -> Result<Vec<Vec<ScanBarFeatures>>, AnalysisError> {
        let width = rgb.width() as i64;
        let height = rgb.height() as i64;
        let nb = num_bars as i64;
        let vertical = vertical_hint.unwrap_or(width > height);

        // Strip dimensions (image_analysis.rs:438..447).
        let bar_w = if vertical {
            (((width as f32) * bar_thickness_frac).max(1.0).round() as i64).min(width)
        } else {
            width
        };
        let bar_h = if vertical {
            height
        } else {
            (((height as f32) * bar_thickness_frac).max(1.0).round() as i64).min(height)
        };

        let steps_count = if num_steps == 0 { 1 } else { num_steps };
        let travel_x = (width - bar_w).max(0);
        let travel_y = (height - bar_h).max(0);

        let mut steps: Vec<Vec<ScanBarFeatures>> = Vec::with_capacity(steps_count);

        for s in 0..steps_count {
            // Bar top-left for this step (image_analysis.rs:456..474).
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

            let bar_rect = Rect {
                x: x0 as u32,
                y: y0 as u32,
                w: if vertical { bar_w } else { width } as u32,
                h: if vertical { height } else { bar_h } as u32,
            };

            // Split the bar rect into `num_bars` sections perpendicular to scan dir
            // (image_analysis.rs:483..505).
            let mut sections: Vec<ScanBarFeatures> = Vec::with_capacity(num_bars);
            for i in 0..num_bars {
                let ii = i as i64;
                let section = if vertical {
                    // Split height into horizontal stripes.
                    let per_h = ((bar_rect.h as i64) / nb).max(1);
                    let y_i = bar_rect.y as i64 + ii * per_h;
                    let h = if i + 1 == num_bars {
                        (bar_rect.y as i64 + bar_rect.h as i64) - y_i
                    } else {
                        per_h
                    }
                    .max(1);
                    Rect {
                        x: bar_rect.x,
                        y: y_i as u32,
                        w: bar_rect.w,
                        h: h as u32,
                    }
                } else {
                    // Split width into vertical slices.
                    let per_w = ((bar_rect.w as i64) / nb).max(1);
                    let x_i = bar_rect.x as i64 + ii * per_w;
                    let w = if i + 1 == num_bars {
                        (bar_rect.x as i64 + bar_rect.w as i64) - x_i
                    } else {
                        per_w
                    }
                    .max(1);
                    Rect {
                        x: x_i as u32,
                        y: bar_rect.y,
                        w: w as u32,
                        h: bar_rect.h,
                    }
                };

                // Clip the rect to the image bounds before cropping (the OpenCV ROI
                // implicitly stays in-bounds because the geometry is derived from the
                // image dims; we clip defensively so crop_imm never exceeds bounds).
                let cw = section.w.min(rgb.width().saturating_sub(section.x)).max(1);
                let ch = section.h.min(rgb.height().saturating_sub(section.y)).max(1);
                // crop_imm yields a zero-copy SubImage view; `.to_image()` materializes
                // the owned section buffer `analyze_section_pure` consumes (SubImage does
                // not impl GenericImageView in image 0.24).
                let section_buf =
                    image::imageops::crop_imm(rgb, section.x, section.y, cw, ch).to_image();
                let feats = analyze_section_pure(&section_buf, i)?;
                sections.push(feats);
            }
            steps.push(sections);
        }

        Ok(steps)
    }

    /// Borrow the precomputed whole-image features.
    pub fn global(&self) -> &GlobalFeatures {
        &self.global
    }

    /// Total scan steps precomputed.
    pub fn steps_len(&self) -> usize {
        self.steps.len()
    }
}

impl FeatureSource for PureAnalysisSource {
    fn global_features(&self) -> GlobalFeatures {
        self.global
    }

    fn scan_bar_features(&self, step_idx: usize, num_instruments: usize) -> Vec<ScanBarFeatures> {
        // Mirror the CannedSource/PrecomputedSource discipline: index the precomputed
        // row, then size it to exactly `num_instruments` (truncate or pad with a
        // neutral zero-bar) so the engine always gets a full row.
        let mut row = self.steps.get(step_idx).cloned().unwrap_or_default();
        row.truncate(num_instruments);
        while row.len() < num_instruments {
            let idx = row.len();
            row.push(ScanBarFeatures {
                bar_index: idx,
                avg_hue: 0.0,
                avg_saturation: 0.0,
                avg_brightness: 0.0,
                edge_density: 0.0,
                texture_laplacian_var: 0.0,
                hue_hist: vec![0.0; 8],
            });
        }
        row
    }

    fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    /// Build a solid-color RGB image of the given size.
    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(w, h, Rgb(rgb))
    }

    // ── rgb_to_hsv: known color conversions ──────────────────────────────────

    #[test]
    fn rgb_to_hsv_pure_red() {
        let (h, s, v) = rgb_to_hsv(Rgb([255, 0, 0]));
        assert!(h.abs() < 1e-3, "pure red hue ≈ 0°, got {h}");
        assert!(
            (s - 100.0).abs() < 1e-3,
            "pure red saturation = 100, got {s}"
        );
        assert!((v - 100.0).abs() < 1e-3, "pure red value = 100, got {v}");
    }

    #[test]
    fn rgb_to_hsv_pure_green_and_blue() {
        let (hg, _, _) = rgb_to_hsv(Rgb([0, 255, 0]));
        assert!((hg - 120.0).abs() < 1e-2, "green hue ≈ 120°, got {hg}");
        let (hb, _, _) = rgb_to_hsv(Rgb([0, 0, 255]));
        assert!((hb - 240.0).abs() < 1e-2, "blue hue ≈ 240°, got {hb}");
    }

    #[test]
    fn rgb_to_hsv_grays_have_zero_saturation() {
        for level in [0u8, 64, 128, 200, 255] {
            let (_h, s, v) = rgb_to_hsv(Rgb([level, level, level]));
            assert!(s.abs() < 1e-3, "gray saturation = 0, got {s} at {level}");
            let expect_v = (level as f32 / 255.0) * 100.0;
            assert!((v - expect_v).abs() < 1e-3, "gray value at {level}");
        }
    }

    // ── hsv_means: solid-color images → exact known means ────────────────────

    #[test]
    fn hsv_means_solid_red_circular() {
        let img = solid(16, 16, [255, 0, 0]);
        let (h, s, v) = hsv_means(img.pixels().copied(), false);
        // All pixels at hue 0 → circular mean ≈ 0 (mod 360).
        assert!(h < 0.5 || h > 359.5, "solid red mean hue ≈ 0°, got {h}");
        assert!((s - 100.0).abs() < 1e-2);
        assert!((v - 100.0).abs() < 1e-2);
    }

    #[test]
    fn hsv_means_solid_blue() {
        let img = solid(8, 8, [0, 0, 255]);
        let (h, _s, _v) = hsv_means(img.pixels().copied(), false);
        assert!(
            (h - 240.0).abs() < 0.5,
            "solid blue mean hue ≈ 240°, got {h}"
        );
    }

    #[test]
    fn hsv_means_circular_vs_arithmetic_at_red_wrap() {
        // Half the pixels hue ≈ 0°, half hue ≈ 358° (both "red"). Arithmetic mean
        // pulls toward 180 (cyan — wrong); circular mean stays near 0/360 (right).
        let mut img = RgbImage::new(2, 1);
        img.put_pixel(0, 0, Rgb([255, 0, 0])); // hue 0
        img.put_pixel(1, 0, Rgb([255, 0, 8])); // hue ≈ 358
        let (h_circ, _, _) = hsv_means(img.pixels().copied(), false);
        let (h_arith, _, _) = hsv_means(img.pixels().copied(), true);
        assert!(
            h_circ < 5.0 || h_circ > 355.0,
            "circular mean stays near red, got {h_circ}"
        );
        assert!(
            h_arith > 170.0 && h_arith < 190.0,
            "arithmetic mean is pulled to ~180 (the wrap bug), got {h_arith}"
        );
    }

    // ── edge_density: black/white edge vs flat field ─────────────────────────

    #[test]
    fn edge_density_flat_field_is_zero() {
        let gray = to_gray(&solid(32, 32, [128, 128, 128]));
        let d = edge_density_pure(&gray);
        assert!(d < 1e-6, "flat gray field has ~zero edge density, got {d}");
    }

    #[test]
    fn edge_density_hard_edge_is_positive() {
        // Left half black, right half white → a strong vertical edge.
        let mut img = RgbImage::new(40, 40);
        for y in 0..40 {
            for x in 0..40 {
                let c = if x < 20 { 0u8 } else { 255u8 };
                img.put_pixel(x, y, Rgb([c, c, c]));
            }
        }
        let gray = to_gray(&img);
        let edge = edge_density_pure(&gray);
        let flat = edge_density_pure(&to_gray(&solid(40, 40, [128, 128, 128])));
        assert!(edge > flat, "hard edge density {edge} > flat {flat}");
        assert!(edge > 0.0, "hard edge produces non-zero edge density");
    }

    // ── laplacian variance: flat ≈ 0, textured > flat ────────────────────────

    #[test]
    fn laplacian_var_flat_is_zero() {
        let gray = to_gray(&solid(24, 24, [100, 100, 100]));
        let v = laplacian_var_pure(&gray);
        assert!(v < 1e-6, "flat field laplacian variance ~0, got {v}");
    }

    #[test]
    fn laplacian_var_checkerboard_is_high() {
        // 1px checkerboard → maximal local 2nd-derivative energy.
        let mut img = RgbImage::new(24, 24);
        for y in 0..24 {
            for x in 0..24 {
                let c = if (x + y) % 2 == 0 { 0u8 } else { 255u8 };
                img.put_pixel(x, y, Rgb([c, c, c]));
            }
        }
        let textured = laplacian_var_pure(&to_gray(&img));
        let flat = laplacian_var_pure(&to_gray(&solid(24, 24, [128, 128, 128])));
        assert!(textured > flat, "checkerboard var {textured} > flat {flat}");
        assert!(
            textured > 1000.0,
            "checkerboard var is large, got {textured}"
        );
    }

    // ── hue histogram: solid color concentrates in one bin, sums to 1 ─────────

    #[test]
    fn hue_histogram_solid_concentrates_and_normalizes() {
        let hist = hue_histogram_pure(solid(8, 8, [255, 0, 0]).pixels().copied(), 8);
        assert_eq!(hist.len(), 8);
        let sum: f32 = hist.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "histogram normalized to 1, got {sum}"
        );
        // Pure red (hue 0 → bin 0) → bin 0 holds (nearly) all mass.
        assert!(
            hist[0] > 0.99,
            "solid red concentrates in bin 0, got {:?}",
            hist
        );
    }

    // ── hue_spread: flat hue ≈ 0 spread; multi-hue > flat ─────────────────────

    #[test]
    fn hue_spread_flat_is_small_multi_is_larger() {
        let flat = hue_spread_pure(solid(16, 16, [200, 0, 0]).pixels().copied());
        // A mix of red/green/blue pixels → wide hue dispersion.
        let mut img = RgbImage::new(3, 1);
        img.put_pixel(0, 0, Rgb([255, 0, 0]));
        img.put_pixel(1, 0, Rgb([0, 255, 0]));
        img.put_pixel(2, 0, Rgb([0, 0, 255]));
        let spread = hue_spread_pure(img.pixels().copied());
        assert!(flat < 1e-3, "flat-hue spread ≈ 0, got {flat}");
        assert!(spread > flat, "multi-hue spread {spread} > flat {flat}");
    }

    // ── shape_complexity: returns count/1000, monotone-ish on blob count ──────

    #[test]
    fn shape_complexity_blank_vs_blobs() {
        // A single white blob on black → ≥1 component.
        let mut one = RgbImage::from_pixel(20, 20, Rgb([0, 0, 0]));
        for y in 6..14 {
            for x in 6..14 {
                one.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let c_one = shape_complexity_pure(&to_gray(&one));
        // Two separated white blobs → more components than one.
        let mut two = RgbImage::from_pixel(20, 20, Rgb([0, 0, 0]));
        for y in 2..6 {
            for x in 2..6 {
                two.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        for y in 14..18 {
            for x in 14..18 {
                two.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
        let c_two = shape_complexity_pure(&to_gray(&two));
        assert!(c_one > 0.0, "one blob → ≥1 component, got {c_one}");
        assert!(
            c_two >= c_one,
            "two blobs ≥ one blob count: {c_two} vs {c_one}"
        );
    }

    // ── analyze_global_pure: field ranges on a known image ───────────────────

    #[test]
    fn analyze_global_pure_ranges_on_solid() {
        let img = solid(32, 24, [0, 128, 255]); // an orange-ish/azure color
        let g = analyze_global_pure(&img).expect("global ok");
        assert!(g.avg_hue >= 0.0 && g.avg_hue <= 360.0, "hue in range");
        assert!(
            g.avg_saturation >= 0.0 && g.avg_saturation <= 100.0,
            "sat in range"
        );
        assert!(
            g.avg_brightness >= 0.0 && g.avg_brightness <= 100.0,
            "bright in range"
        );
        assert!(
            g.edge_density >= 0.0 && g.edge_density <= 1.0,
            "edge in 0..1"
        );
        assert!(g.edge_density < 1e-6, "solid color → ~0 edge density");
        assert!(
            (g.aspect_ratio - (32.0 / 24.0)).abs() < 1e-5,
            "aspect = w/h"
        );
        assert!(g.texture_laplacian_var < 1e-6, "solid → ~0 texture var");
    }

    #[test]
    fn analyze_global_pure_empty_errors() {
        let img = RgbImage::new(0, 0);
        let err = analyze_global_pure(&img).unwrap_err();
        assert!(matches!(err, AnalysisError::EmptyImage(_)));
    }

    // ── analyze_section_pure: produces a ScanBarFeatures with the bar_index ───

    #[test]
    fn analyze_section_pure_fills_fields() {
        let img = solid(16, 16, [0, 255, 0]);
        let view = image::imageops::crop_imm(&img, 0, 0, 16, 16).to_image();
        let sb = analyze_section_pure(&view, 3).expect("section ok");
        assert_eq!(sb.bar_index, 3);
        assert!((sb.avg_hue - 120.0).abs() < 1.0, "green section hue ≈ 120");
        assert_eq!(sb.hue_hist.len(), 8);
        assert!(sb.edge_density < 1e-6, "solid section → ~0 edge density");
    }

    // ── PureAnalysisSource: FeatureSource contract + geometry parity shape ────

    /// A horizontally-striped test image so the per-section rows differ, exercising
    /// the scan geometry. Top band red, bottom band blue.
    fn striped(w: u32, h: u32) -> RgbImage {
        let mut img = RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let c = if y < h / 2 { [255, 0, 0] } else { [0, 0, 255] };
                img.put_pixel(x, y, Rgb(c));
            }
        }
        img
    }

    #[test]
    fn pure_source_satisfies_feature_source_contract() {
        let img = PureImage::from_rgb(striped(64, 48));
        let src = PureAnalysisSource::extract(&img, 4, 0.10, 6, Some(false)).expect("extract ok");

        // step_count matches the requested number of steps.
        assert_eq!(src.step_count(), 6, "step_count = num_steps");

        // Each row is exactly num_instruments wide (the engine's contract).
        for step in 0..src.step_count() {
            let row = src.scan_bar_features(step, 4);
            assert_eq!(row.len(), 4, "row has one bar per instrument");
            // bar_index is dense 0..num_instruments.
            for (i, b) in row.iter().enumerate() {
                assert_eq!(b.bar_index, i);
            }
        }

        // Out-of-range step → padded zero-bars, never a panic.
        let pad = src.scan_bar_features(999, 4);
        assert_eq!(pad.len(), 4);
        assert!(pad.iter().all(|b| b.edge_density == 0.0));

        // global features are well-formed.
        let g = src.global_features();
        assert!(g.aspect_ratio > 0.0);
    }

    #[test]
    fn pure_source_truncates_and_pads_to_num_instruments() {
        let img = PureImage::from_rgb(solid(40, 40, [128, 64, 32]));
        let src = PureAnalysisSource::extract(&img, 4, 0.10, 3, Some(false)).expect("extract");
        // Ask for fewer than extracted → truncates.
        let narrow = src.scan_bar_features(0, 2);
        assert_eq!(narrow.len(), 2);
        // Ask for more than extracted → pads.
        let wide = src.scan_bar_features(0, 6);
        assert_eq!(wide.len(), 6);
    }

    #[test]
    fn pure_source_zero_instruments_errors() {
        let img = PureImage::from_rgb(solid(8, 8, [10, 20, 30]));
        let err = PureAnalysisSource::extract(&img, 0, 0.10, 2, None).unwrap_err();
        assert!(matches!(err, AnalysisError::ZeroBars));
    }

    #[test]
    fn pure_source_num_steps_zero_yields_one_step() {
        let img = PureImage::from_rgb(solid(16, 16, [50, 60, 70]));
        let src = PureAnalysisSource::extract(&img, 2, 0.10, 0, None).expect("extract");
        assert_eq!(src.step_count(), 1, "num_steps=0 → exactly one step");
    }

    /// Compile-time proof: PureAnalysisSource is consumable wherever the engine
    /// wants an `&impl FeatureSource` (the contract the engine core uses).
    #[test]
    fn pure_source_is_usable_as_feature_source_generic() {
        fn takes_source<S: FeatureSource>(s: &S) -> usize {
            s.step_count()
        }
        let img = PureImage::from_rgb(solid(20, 20, [0, 128, 200]));
        let src = PureAnalysisSource::extract(&img, 3, 0.10, 5, None).expect("extract");
        assert_eq!(takes_source(&src), 5);
    }
}

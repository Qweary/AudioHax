// src/image_analysis.rs
use anyhow::{anyhow, Result};
use opencv::{core, imgproc, prelude::*};
use std::mem;

/// The set of features we extract at three levels:
/// - GlobalFeatures: entire image
/// - ScanBarFeatures: averaged across the scan bar (used to pick chords/progression)
/// - LocalFeatures: per-section features used to pick actual notes/velocity/articulation
#[derive(Debug, Clone)]
pub struct GlobalFeatures {
    pub avg_hue: f32,            // 0..360
    pub avg_saturation: f32,     // 0..100
    pub avg_brightness: f32,     // 0..100 (value in HSV)
    pub edge_density: f32,       // 0..1 proportion of edge pixels
    pub hue_spread: f32,         // measure of how spread the hues are (0..1)
    pub texture_laplacian_var: f32, // variance of Laplacian (focus/texture)
    pub shape_complexity: f32,   // crude metric: number of contours normalized
    pub aspect_ratio: f32,       // width / height of image
}

#[derive(Debug, Clone)]
pub struct ScanBarFeatures {
    pub bar_index: usize,
    pub avg_hue: f32,
    pub avg_saturation: f32,
    pub avg_brightness: f32,
    pub edge_density: f32,
    pub texture_laplacian_var: f32,
    pub hue_hist: Vec<f32>, // optional small histogram for fingerprinting
}

/// Local features returned for a small region.  Enhanced to include
/// saturation/brightness so scan-bar aggregation can compute those cleanly.
#[derive(Debug, Clone)]
pub struct LocalFeatures {
    pub avg_hue: f32,
    pub avg_saturation: f32,
    pub avg_brightness: f32,
    pub hue_delta_from_bar: f32,
    pub brightness_delta_from_bar: f32,
    pub edge_sharpness: f32,
    pub edge_orientation_bias: f32, // -1..1 (negative = vertical bias, positive = horizontal bias)
    pub texture_complexity: f32,
    pub contour_circularity: f32, // 0..1 for dominant contour
}

/// Helper to produce a "zero" AlgorithmHint value expected by some opencv bindings.
/// This uses zeroed memory for the enum-like FFI type; it's a pragmatic minimal approach
/// for passing a neutral hint value to functions like cvt_color.
fn default_algorithm_hint() -> core::AlgorithmHint {
    // NOTE: AlgorithmHint is an FFI enum-like POD; zeroed works here as a neutral hint.
    unsafe { mem::zeroed() }
}

/// High-level: analyze entire image and return GlobalFeatures.
pub fn analyze_global(image: &Mat) -> Result<GlobalFeatures> {
    if image.empty() {
        return Err(anyhow!("Empty image passed to analyze_global"));
    }

    // Convert to HSV (this binding expects 5 args; last is AlgorithmHint)
    let mut hsv = Mat::default();
    imgproc::cvt_color(
        image,
        &mut hsv,
        imgproc::COLOR_BGR2HSV,
        0,
        default_algorithm_hint(),
    )?;

    // Split channels into a core::Vector<Mat>
    let mut channels = core::Vector::<Mat>::new();
    core::split(&hsv, &mut channels)?;

    let h = channels.get(0)?;
    let s = channels.get(1)?;
    let v = channels.get(2)?;

    // mean/stddev for H,S,V (note: H in OpenCV is 0..179; convert to degrees)
    let mut mean_h = core::Scalar::default();
    let mut stddev_h = core::Scalar::default();
    core::mean_std_dev(&h, &mut mean_h, &mut stddev_h, &core::no_array())?;

    let mut mean_s = core::Scalar::default();
    let mut stddev_s = core::Scalar::default();
    core::mean_std_dev(&s, &mut mean_s, &mut stddev_s, &core::no_array())?;

    let mut mean_v = core::Scalar::default();
    let mut stddev_v = core::Scalar::default();
    core::mean_std_dev(&v, &mut mean_v, &mut stddev_v, &core::no_array())?;

    // convert H to 0..360 degrees, S/V to 0..100 percent
    let avg_hue = (mean_h[0] as f32) * 2.0;
    let avg_saturation = (mean_s[0] as f32) * (100.0 / 255.0);
    let avg_brightness = (mean_v[0] as f32) * (100.0 / 255.0);

    // Edge density using Canny
    let mut gray = Mat::default();
    imgproc::cvt_color(
        image,
        &mut gray,
        imgproc::COLOR_BGR2GRAY,
        0,
        default_algorithm_hint(),
    )?;
    let mut edges = Mat::default();
    imgproc::canny(&gray, &mut edges, 50.0, 150.0, 3, false)?;
    let edge_count = core::count_non_zero(&edges)?;
    let total_pixels = (edges.rows() * edges.cols()) as f32;
    let edge_density = if total_pixels > 0.0 { (edge_count as f32) / total_pixels } else { 0.0 };

    // Laplacian variance (measure of texture / focus)
    let mut lap = Mat::default();
    imgproc::laplacian(&gray, &mut lap, core::CV_64F, 3, 1.0, 0.0, core::BORDER_DEFAULT)?;
    let mut mean_lap = core::Scalar::default();
    let mut stddev_lap = core::Scalar::default();
    core::mean_std_dev(&lap, &mut mean_lap, &mut stddev_lap, &core::no_array())?;
    let lap_var = stddev_lap[0] as f32 * stddev_lap[0] as f32;

    // Contour count and largest contour circularity
    let mut thresh = Mat::default();
    imgproc::threshold(
        &gray,
        &mut thresh,
        0.0,
        255.0,
        imgproc::THRESH_OTSU | imgproc::THRESH_BINARY,
    )?;
    let mut contours = core::Vector::<core::Vector::<core::Point>>::new();
    imgproc::find_contours(
        &thresh,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::new(0, 0),
    )?;
    let contour_count = contours.len() as f32;
    let shape_complexity = contour_count / 1000.0; // normalize (adjust heuristic as needed)

    // largest contour circularity
    let mut max_circularity = 0.0f32;
    for i in 0..contours.len() {
        let cnt = contours.get(i)?;
        let area = imgproc::contour_area(&cnt, false)?;
        if area <= 0.0 {
            continue;
        }
        let perimeter = imgproc::arc_length(&cnt, true)?;
        if perimeter > 0.0 {
            let circularity =
                (4.0 * std::f64::consts::PI * (area as f64)) / (perimeter * perimeter);
            if circularity as f32 > max_circularity {
                max_circularity = circularity as f32;
            }
        }
    }

    let aspect_ratio = (image.cols() as f32) / (image.rows() as f32);

    // hue spread: approximate via standard deviation of h channel (heuristic)
    let hue_spread = stddev_h[0] as f32 / 90.0;

    Ok(GlobalFeatures {
        avg_hue,
        avg_saturation,
        avg_brightness,
        edge_density,
        hue_spread,
        texture_laplacian_var: lap_var,
        shape_complexity,
        aspect_ratio,
    })
}

/// Analyze the full scan bar by slicing the image along the chosen axis.
///
/// - `num_bars`: number of instrument subdivisions in the bar
/// - `vertical`: if true, bar runs top->bottom and slices are vertical strips; if false, horizontal bar slices
pub fn analyze_scan_bar(image: &Mat, num_bars: usize, vertical: bool) -> Result<Vec<ScanBarFeatures>> {
    if image.empty() {
        return Err(anyhow!("Empty image passed to analyze_scan_bar"));
    }
    if num_bars == 0 {
        return Err(anyhow!("num_bars must be > 0"));
    }

    let mut results: Vec<ScanBarFeatures> = Vec::new();
    // cols() and rows() return i32 in opencv crate
    let (width, height) = (image.cols(), image.rows());
    let nb = num_bars as i32;

    for i in 0..num_bars {
        let ii = i as i32;
        let roi = if vertical {
            let x0 = (ii * width) / nb;
            let x1 = ((ii + 1) * width) / nb;
            let w = (x1 - x0).max(1);
            core::Rect::new(x0, 0, w, height)
        } else {
            let y0 = (ii * height) / nb;
            let y1 = ((ii + 1) * height) / nb;
            let h = (y1 - y0).max(1);
            core::Rect::new(0, y0, width, h)
        };

        // Mat::roi returns a BoxedRef; use try_clone() to obtain an owned Mat
        let sub_roi = Mat::roi(image, roi)?;
        let sub = sub_roi.try_clone()?; // owned Mat

        // Use the enhanced local analysis to get saturation/brightness and other metrics
        let lf = analyze_local_basic(&sub)?;
        let hue_hist = compute_hue_histogram(&sub, 8)?;

        results.push(ScanBarFeatures {
            bar_index: i,
            avg_hue: lf.avg_hue,
            avg_saturation: lf.avg_saturation,
            avg_brightness: lf.avg_brightness,
            edge_density: lf.edge_sharpness,
            texture_laplacian_var: lf.texture_complexity,
            hue_hist,
        });
    }

    Ok(results)
}

/// Analyze a small region and return LocalFeatures (enhanced to include sat/bright).
pub fn analyze_local_basic(region: &Mat) -> Result<LocalFeatures> {
    if region.empty() {
        return Err(anyhow!("Empty region passed to analyze_local_basic"));
    }

    // Convert to HSV and compute means
    let mut hsv = Mat::default();
    imgproc::cvt_color(
        region,
        &mut hsv,
        imgproc::COLOR_BGR2HSV,
        0,
        default_algorithm_hint(),
    )?;
    let mut channels = core::Vector::<Mat>::new();
    core::split(&hsv, &mut channels)?;
    let h = channels.get(0)?;
    let s = channels.get(1)?;
    let v = channels.get(2)?;

    let mut mean_h = core::Scalar::default();
    let mut stddev_h = core::Scalar::default();
    core::mean_std_dev(&h, &mut mean_h, &mut stddev_h, &core::no_array())?;
    let mut mean_s = core::Scalar::default();
    let mut stddev_s = core::Scalar::default();
    core::mean_std_dev(&s, &mut mean_s, &mut stddev_s, &core::no_array())?;
    let mut mean_v = core::Scalar::default();
    let mut stddev_v = core::Scalar::default();
    core::mean_std_dev(&v, &mut mean_v, &mut stddev_v, &core::no_array())?;

    let avg_hue = (mean_h[0] as f32) * 2.0;
    let avg_saturation = (mean_s[0] as f32) * (100.0 / 255.0);
    let avg_brightness = (mean_v[0] as f32) * (100.0 / 255.0);

    // Edge density
    let mut gray = Mat::default();
    imgproc::cvt_color(
        region,
        &mut gray,
        imgproc::COLOR_BGR2GRAY,
        0,
        default_algorithm_hint(),
    )?;
    let mut edges = Mat::default();
    imgproc::canny(&gray, &mut edges, 50.0, 150.0, 3, false)?;
    let edge_count = core::count_non_zero(&edges)?;
    let total_pixels = (edges.rows() * edges.cols()) as f32;
    let edge_density = if total_pixels > 0.0 { (edge_count as f32) / total_pixels } else { 0.0 };

    // Edge orientation bias using Sobel gradients: compute mean of gradients
    let mut grad_x = Mat::default();
    let mut grad_y = Mat::default();
    imgproc::sobel(&gray, &mut grad_x, core::CV_32F, 1, 0, 3, 1.0, 0.0, core::BORDER_DEFAULT)?;
    imgproc::sobel(&gray, &mut grad_y, core::CV_32F, 0, 1, 3, 1.0, 0.0, core::BORDER_DEFAULT)?;
    let mut mean_gx = core::Scalar::default();
    let mut std_gx = core::Scalar::default();
    core::mean_std_dev(&grad_x, &mut mean_gx, &mut std_gx, &core::no_array())?;
    let mut mean_gy = core::Scalar::default();
    let mut std_gy = core::Scalar::default();
    core::mean_std_dev(&grad_y, &mut mean_gy, &mut std_gy, &core::no_array())?;
    // If |gx| > |gy| -> horizontal bias, else vertical
    let edge_orientation_bias =
        (mean_gx[0] as f32 - mean_gy[0] as f32) / ((mean_gx[0].abs() + mean_gy[0].abs()) as f32 + 1e-6);

    // Laplacian variance
    let mut lap = Mat::default();
    imgproc::laplacian(&gray, &mut lap, core::CV_64F, 3, 1.0, 0.0, core::BORDER_DEFAULT)?;
    let mut mean_lap = core::Scalar::default();
    let mut stddev_lap = core::Scalar::default();
    core::mean_std_dev(&lap, &mut mean_lap, &mut stddev_lap, &core::no_array())?;
    let lap_var = stddev_lap[0] as f32 * stddev_lap[0] as f32;

    // Contours -> circularity of largest contour (estimate)
    let mut thresh = Mat::default();
    imgproc::threshold(
        &gray,
        &mut thresh,
        0.0,
        255.0,
        imgproc::THRESH_OTSU | imgproc::THRESH_BINARY,
    )?;
    let mut contours = core::Vector::<core::Vector::<core::Point>>::new();
    imgproc::find_contours(
        &thresh,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::new(0, 0),
    )?;

    let mut max_circularity = 0.0f32;
    for i in 0..contours.len() {
        let cnt = contours.get(i)?;
        let area = imgproc::contour_area(&cnt, false)?;
        if area <= 0.0 {
            continue;
        }
        let perimeter = imgproc::arc_length(&cnt, true)?;
        if perimeter > 0.0 {
            let circularity =
                (4.0 * std::f64::consts::PI * (area as f64)) / (perimeter * perimeter);
            if circularity as f32 > max_circularity {
                max_circularity = circularity as f32;
            }
        }
    }

    Ok(LocalFeatures {
        avg_hue,
        avg_saturation,
        avg_brightness,
        hue_delta_from_bar: 0.0, // caller will compute real deltas if desired
        brightness_delta_from_bar: 0.0,
        edge_sharpness: edge_density,
        edge_orientation_bias,
        texture_complexity: lap_var,
        contour_circularity: max_circularity,
    })
}

/// Helper: compute small hue histogram (N bins) normalized to 0..1
fn compute_hue_histogram(image: &Mat, bins: i32) -> Result<Vec<f32>> {
    // convert to HSV
    let mut hsv = Mat::default();
    imgproc::cvt_color(
        image,
        &mut hsv,
        imgproc::COLOR_BGR2HSV,
        0,
        default_algorithm_hint(),
    )?;
    let mut channels = core::Vector::<Mat>::new();
    core::split(&hsv, &mut channels)?;
    let h = channels.get(0)?;

    // Prepare histogram parameters
    let mut hist_size = core::Vector::<i32>::new();
    hist_size.push(bins);

    // ranges must be Vector<f32> for calc_hist in this binding
    let mut ranges = core::Vector::<f32>::new();
    ranges.push(0.0f32);
    ranges.push(180.0f32); // OpenCV H range

    let mut hist = Mat::default();
    let mut images = core::Vector::<Mat>::new();
    // clone() returns Mat (not Result)
    images.push(h.clone());

    // channels for calc_hist: vector with single element 0
    let mut chs = core::Vector::<i32>::new();
    chs.push(0);

    imgproc::calc_hist(
        &images,
        &chs,
        &Mat::default(),
        &mut hist,
        &hist_size,
        &ranges,
        false,
    )?;

    // read histogram bins as f32
    let mut out: Vec<f32> = Vec::with_capacity(bins as usize);
    let mut sum = 0f32;
    for b in 0..bins {
        let val = match hist.at_2d::<f32>(b, 0) {
            Ok(v) => *v,
            Err(_) => 0.0,
        };
        out.push(val);
        sum += val;
    }

    if sum > 0.0 {
        for v in out.iter_mut() {
            *v /= sum;
        }
    }

    Ok(out)
}

/// Draws an overlay showing the scan bar and its instrument subdivisions.
/// - `vertical`: if true, scan bar is vertical, split into vertical strips
/// - `num_bars`: how many instrument subdivisions
/// Returns: a cloned Mat with overlays drawn.
pub fn draw_scan_bar_overlay(image: &Mat, num_bars: usize, vertical: bool) -> Result<Mat> {
    if image.empty() {
        return Err(anyhow!("Empty image passed to draw_scan_bar_overlay"));
    }
    if num_bars == 0 {
        return Err(anyhow!("num_bars must be > 0"));
    }

    // clone returns Mat (not Result)
    let mut overlay = image.clone();
    let (width, height) = (overlay.cols(), overlay.rows());
    let nb = num_bars as i32;

    // Draw outer rectangle for scan bar
    let scan_bar_rect = core::Rect::new(0, 0, width, height);
    imgproc::rectangle(
        &mut overlay,
        scan_bar_rect,
        core::Scalar::new(0.0, 255.0, 0.0, 0.0), // green
        2,
        imgproc::LINE_8,
        0,
    )?;

    // Draw subdivisions
    for i in 1..num_bars {
        let ii = i as i32;
        if vertical {
            let x = (ii * width) / nb;
            imgproc::line(
                &mut overlay,
                core::Point::new(x, 0),
                core::Point::new(x, height),
                core::Scalar::new(0.0, 0.0, 255.0, 0.0), // red
                1,
                imgproc::LINE_8,
                0,
            )?;
        } else {
            let y = (ii * height) / nb;
            imgproc::line(
                &mut overlay,
                core::Point::new(0, y),
                core::Point::new(width, y),
                core::Scalar::new(0.0, 0.0, 255.0, 0.0), // red
                1,
                imgproc::LINE_8,
                0,
            )?;
        }
    }

    Ok(overlay)
}

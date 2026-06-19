// src/bin/make_tiled_payload.rs
//
// Build a single "tiled payload" binary from an image suitable for sending
// with your existing AHX frame + modem pipeline. The output file contains:
//  - header: "TLF1" magic, levels, tile_size, orig_w, orig_h, manifest_len
//  - JSON manifest (UTF-8) describing tiles (level, tx, ty, w, h, offset, len, avg_color)
//  - concatenated JPEG tile blobs
//
// Usage:
//   cargo run --bin make_tiled_payload -- input.jpg out_payload.bin --tile-size 64 --levels 3 --quality 80
//
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageOutputFormat, RgbImage};
use serde::Serialize;

#[derive(Serialize)]
struct TileEntry {
    level: u8,
    tx: u32,
    ty: u32,
    w: u32,
    h: u32,
    offset: u64,
    len: u64,
    avg_color: [u8; 3],
    encoding: String, // e.g. "jpeg"
}

fn average_color_rgb(buf: &RgbImage) -> [u8; 3] {
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;
    for px in buf.pixels() {
        r_sum += px[0] as u64;
        g_sum += px[1] as u64;
        b_sum += px[2] as u64;
        count += 1;
    }
    if count == 0 {
        return [0, 0, 0];
    }
    [
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
    ]
}

fn encode_jpeg_bytes_from_rgb(buf: &RgbImage, quality: u8) -> Vec<u8> {
    let mut v = Vec::new();
    // Use image crate's JPEG encoder by converting to DynamicImage for convenience
    let dyn_img = DynamicImage::ImageRgb8(buf.clone());
    let mut cursor = std::io::Cursor::new(&mut v);
    dyn_img
        .write_to(&mut cursor, ImageOutputFormat::Jpeg(quality))
        .expect("JPEG encoding failed");
    v
}

fn make_pyramid(img: &DynamicImage, levels: usize) -> Vec<DynamicImage> {
    // produces [fullres, half, quarter, ...]
    let mut out = Vec::with_capacity(levels);
    let (w, h) = img.dimensions();
    for level in 0..levels {
        if level == 0 {
            out.push(img.clone());
        } else {
            let scale = 1.0 / (2u32.pow(level as u32) as f32);
            let nw = (w as f32 * scale).max(1.0).round() as u32;
            let nh = (h as f32 * scale).max(1.0).round() as u32;
            let resized = img.resize_exact(nw, nh, FilterType::Triangle);
            out.push(resized);
        }
    }
    out
}

fn tile_and_encode_level(
    level_img: &DynamicImage,
    tile_size: u32,
    level: u8,
    quality: u8,
) -> (Vec<TileEntry>, Vec<Vec<u8>>) {
    let (w, h) = level_img.dimensions();
    let tx_count = ((w + tile_size - 1) / tile_size) as u32;
    let ty_count = ((h + tile_size - 1) / tile_size) as u32;

    let mut entries: Vec<TileEntry> = Vec::new();
    let mut blobs: Vec<Vec<u8>> = Vec::new();

    for ty in 0..ty_count {
        let y0 = ty * tile_size;
        let th = std::cmp::min(tile_size, h - y0);
        for tx in 0..tx_count {
            let x0 = tx * tile_size;
            let tw = std::cmp::min(tile_size, w - x0);
            // crop_imm returns a DynamicImage
            let sub = level_img.crop_imm(x0, y0, tw, th);
            let rgb = sub.to_rgb8();
            let avg = average_color_rgb(&rgb);
            let blob = encode_jpeg_bytes_from_rgb(&rgb, quality);

            entries.push(TileEntry {
                level,
                tx,
                ty,
                w: tw,
                h: th,
                offset: 0,
                len: blob.len() as u64,
                avg_color: avg,
                encoding: "jpeg".to_string(),
            });
            blobs.push(blob);
        }
    }
    (entries, blobs)
}

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!(
        "  {} <input.jpg> <out_payload.bin> [--tile-size N] [--levels L] [--quality Q]",
        name
    );
    eprintln!("Defaults: tile-size=64, levels=3, quality=80");
    eprintln!();
    eprintln!("Example:");
    eprintln!(
        "  {} image.jpg payload_tiled.bin --tile-size 64 --levels 3 --quality 80",
        name
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let input = &args[1];
    let out = &args[2];

    // defaults
    let mut tile_size: u32 = 64;
    let mut levels: usize = 3;
    let mut quality: u8 = 80;

    let mut i = 3usize;
    while i < args.len() {
        match args[i].as_str() {
            "--tile-size" => {
                if let Some(v) = args.get(i + 1) {
                    tile_size = v.parse::<u32>().unwrap_or(tile_size);
                }
                i += 2;
            }
            "--levels" => {
                if let Some(v) = args.get(i + 1) {
                    levels = v.parse::<usize>().unwrap_or(levels);
                }
                i += 2;
            }
            "--quality" => {
                if let Some(v) = args.get(i + 1) {
                    quality = v.parse::<u8>().unwrap_or(quality);
                }
                i += 2;
            }
            _ => {
                eprintln!("Unknown arg {}", args[i]);
                i += 1;
            }
        }
    }

    // Load image
    if !Path::new(input).exists() {
        eprintln!("Input file not found: {}", input);
        std::process::exit(1);
    }
    let img = image::open(input)?;
    let (orig_w, orig_h) = img.dimensions();

    // Build pyramid
    let pyramid = make_pyramid(&img, levels);
    // We'll construct manifest entries and collect blobs
    let mut manifest_entries: Vec<TileEntry> = Vec::new();
    let mut blobs: Vec<Vec<u8>> = Vec::new();

    // We want coarsest (largest level index) first for progressive preview.
    // pyramid index 0 = fullres, so reverse iteration gives coarsest -> fine
    for lvl_idx in (0..levels).rev() {
        let level_img = &pyramid[lvl_idx];
        // map lvl_idx -> stored level index (0 = coarsest)
        let level = (levels - 1 - lvl_idx) as u8;
        let (mut entries, mut level_blobs) =
            tile_and_encode_level(level_img, tile_size, level, quality);
        manifest_entries.append(&mut entries);
        blobs.append(&mut level_blobs);
    }

    // compute offsets (tile stream starts after manifest)
    let mut offset_cursor: u64 = 0;
    for entry in manifest_entries.iter_mut() {
        entry.offset = offset_cursor;
        offset_cursor += entry.len;
    }

    // final manifest JSON
    let manifest_json = serde_json::to_vec_pretty(&manifest_entries)?;
    let manifest_len = manifest_json.len() as u64;

    // Write header, manifest, blobs
    let mut outf = File::create(out)?;
    // Header layout: "TLF1" | levels(u8) | tile_size(u16 BE) | orig_w(u16 BE) | orig_h(u16 BE) | manifest_len(u32 BE)
    outf.write_all(b"TLF1")?;
    outf.write_all(&[levels as u8])?;
    outf.write_all(&(tile_size as u16).to_be_bytes())?;
    outf.write_all(&(orig_w as u16).to_be_bytes())?;
    outf.write_all(&(orig_h as u16).to_be_bytes())?;
    outf.write_all(&(manifest_len as u32).to_be_bytes())?;
    outf.write_all(&manifest_json)?;

    // write blobs in same order as manifest_entries
    for b in blobs.iter() {
        outf.write_all(b)?;
    }

    println!(
        "Wrote tiled payload: {} (orig {}x{}, levels {}, tile {})",
        out, orig_w, orig_h, levels, tile_size
    );
    println!(
        "Manifest entries: {}, manifest bytes: {}",
        manifest_entries.len(),
        manifest_len
    );
    Ok(())
}

// src/bin/unpack_tiled_payload.rs
//
// Unpack a "TLF1" tiled payload created by make_tiled_payload.rs
// Produces a reconstructed full-resolution image using available tiles and
// falling back to avg_color for missing tiles.
//
// Usage:
//   cargo run --bin unpack_tiled_payload -- payload_tiled.bin out_prefix
//
use std::env;
use std::fs::File;
use std::io::Read;

use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageBuffer, Pixel, RgbImage};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TileEntry {
    level: u8,
    tx: u32,
    ty: u32,
    w: u32,
    h: u32,
    offset: u64,
    len: u64,
    avg_color: [u8; 3],
}

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <payload_tiled.bin> <out_prefix>", name);
    eprintln!("Example:");
    eprintln!("  {} payload_tiled.bin recovered", name);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let payload_path = &args[1];
    let out_prefix = &args[2];

    let mut f = File::open(payload_path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    if buf.len() < 4 || &buf[0..4] != b"TLF1" {
        return Err("Not a TLF1 payload".into());
    }
    let mut cursor = 4usize;
    let levels = buf[cursor] as usize;
    cursor += 1;
    let tile_size = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]) as u32;
    cursor += 2;
    let orig_w = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]) as u32;
    cursor += 2;
    let orig_h = u16::from_be_bytes([buf[cursor], buf[cursor + 1]]) as u32;
    cursor += 2;
    let manifest_len = u32::from_be_bytes([
        buf[cursor],
        buf[cursor + 1],
        buf[cursor + 2],
        buf[cursor + 3],
    ]) as usize;
    cursor += 4;

    if cursor + manifest_len > buf.len() {
        return Err("Manifest truncated".into());
    }
    let manifest_json = &buf[cursor..cursor + manifest_len];
    cursor += manifest_len;
    let tile_stream_start = cursor;

    let manifest: Vec<TileEntry> = serde_json::from_slice(manifest_json)?;

    println!(
        "Payload: levels={}, tile_size={}, orig={}x{}, manifest_entries={}",
        levels,
        tile_size,
        orig_w,
        orig_h,
        manifest.len()
    );

    // Build full-resolution canvas
    let mut canvas: RgbImage = ImageBuffer::from_pixel(orig_w, orig_h, image::Rgb([0u8, 0u8, 0u8]));

    // We'll draw tiles; entries may be in coarsest->fine order.
    // For each manifest entry attempt to decode blob; if fail, paint avg_color region.
    for entry in manifest.iter() {
        let blob_start = tile_stream_start + (entry.offset as usize);
        let blob_end = blob_start + (entry.len as usize);
        if blob_end > buf.len() {
            eprintln!(
                "Tile blob out of bounds (entry {:?}) - using avg color",
                entry
            );
            // paint avg color in the full-res tile region
            paint_avg_on_canvas(&mut canvas, entry, tile_size, levels)?;
            continue;
        }
        let blob = &buf[blob_start..blob_end];
        match image::load_from_memory(blob) {
            Ok(tile_img) => {
                // compute mapping to full-res canvas
                // stored entry.level: 0=coarsest, (levels-1)=fullres as created by packer
                let lvl_idx = (levels - 1).saturating_sub(entry.level as usize); // reverse mapping
                let scale = 2u32.pow(lvl_idx as u32);
                let full_x = entry.tx * tile_size * scale;
                let full_y = entry.ty * tile_size * scale;
                let full_w = entry.w * scale;
                let full_h = entry.h * scale;
                // resize decoded tile to full_w x full_h
                let resized = tile_img.resize_exact(full_w, full_h, FilterType::Triangle);
                // paste into canvas (resized is DynamicImage)
                paste_dynimage_into_rgb(&mut canvas, &resized, full_x, full_y)?;
            }
            Err(e) => {
                eprintln!("Failed decoding tile: {} - using avg color", e);
                paint_avg_on_canvas(&mut canvas, entry, tile_size, levels)?;
            }
        }
    }

    // Save final assembled image
    let out_path = format!("{}_recon.jpg", out_prefix);
    canvas.save(&out_path)?;
    println!("Wrote reconstructed image: {}", out_path);

    Ok(())
}

fn paste_dynimage_into_rgb(
    canvas: &mut RgbImage,
    img: &DynamicImage,
    x: u32,
    y: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h) = img.dimensions();
    for yy in 0..h {
        for xx in 0..w {
            let px = img.get_pixel(xx, yy).to_rgb();
            let cx = x + xx;
            let cy = y + yy;
            if cx < canvas.width() && cy < canvas.height() {
                canvas.put_pixel(cx, cy, image::Rgb([px[0], px[1], px[2]]));
            }
        }
    }
    Ok(())
}

fn paint_avg_on_canvas(
    canvas: &mut RgbImage,
    entry: &TileEntry,
    tile_size: u32,
    levels: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let lvl_idx = (levels - 1).saturating_sub(entry.level as usize);
    let scale = 2u32.pow(lvl_idx as u32);
    let full_x = entry.tx * tile_size * scale;
    let full_y = entry.ty * tile_size * scale;
    let full_w = (entry.w * scale) as u32;
    let full_h = (entry.h * scale) as u32;
    for yy in 0..full_h {
        for xx in 0..full_w {
            let cx = full_x + xx;
            let cy = full_y + yy;
            if cx < canvas.width() && cy < canvas.height() {
                canvas.put_pixel(cx, cy, image::Rgb(entry.avg_color));
            }
        }
    }
    Ok(())
}

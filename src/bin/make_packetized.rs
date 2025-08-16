// src/bin/make_packetized.rs
use std::env;
use std::fs;
use std::io::Write;

use audiohax::modem::{self, ModemParams};

fn print_usage(name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <input_file> <out_packetized.bin> [--compress] [--rs-data D --rs-parity P --rs-shard-size S] [--pkt-size N --repeats R]", name);
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} myimage.jpg packetized.bin --compress --rs-data 4 --rs-parity 2 --rs-shard-size 128", name);
    eprintln!("  {} myfile.bin packetized.bin --pkt-size 200 --repeats 3", name);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let input_path = &args[1];
    let out_path = &args[2];

    let mut compress = false;
    let mut rs_data: Option<usize> = None;
    let mut rs_parity: Option<usize> = None;
    let mut rs_shard_size: usize = 128;
    let mut pkt_size: usize = 200;
    let mut repeats: usize = 3;

    let mut i = 3usize;
    while i < args.len() {
        match args[i].as_str() {
            "--compress" => { compress = true; i += 1; }
            "--rs-data" => { if let Some(v) = args.get(i+1) { rs_data = v.parse::<usize>().ok(); } i += 2; }
            "--rs-parity" => { if let Some(v) = args.get(i+1) { rs_parity = v.parse::<usize>().ok(); } i += 2; }
            "--rs-shard-size" => { if let Some(v) = args.get(i+1) { rs_shard_size = v.parse::<usize>().unwrap_or(128); } i += 2; }
            "--pkt-size" => { if let Some(v) = args.get(i+1) { pkt_size = v.parse::<usize>().unwrap_or(200); } i += 2; }
            "--repeats" => { if let Some(v) = args.get(i+1) { repeats = v.parse::<usize>().unwrap_or(3); } i += 2; }
            _ => { eprintln!("Unknown arg {}", args[i]); i += 1; }
        }
    }

    let payload = fs::read(input_path)?;
    let filename = std::path::Path::new(input_path).file_name().and_then(|s| s.to_str()).unwrap_or("payload");

    // Build frame header+payload (same as modem_encode)
    let frame = modem::build_frame(
        filename,
        &payload,
        compress,
        None, // no encryption here; add CLI option if you want
    )?;

    println!("Built frame: {} bytes (payload {})", frame.len(), payload.len());

    let packetized = if let (Some(d), Some(p)) = (rs_data, rs_parity) {
        println!("Packetizing with RS: data={} parity={} shard_size={}", d, p, rs_shard_size);
        modem::packetize_stream_rs(&frame, rs_shard_size, d, p)?
    } else {
        println!("Packetizing with repeats: pkt_size={} repeats={}", pkt_size, repeats);
        modem::packetize_stream(&frame, pkt_size, repeats)
    };

    let mut f = std::fs::File::create(out_path)?;
    f.write_all(&packetized)?;
    println!("Wrote {} bytes to {}", packetized.len(), out_path);
    Ok(())
}

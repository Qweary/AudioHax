// src/bin/make_packetized.rs
//
// S9 (WS-4 Phase 1): hand-rolled positional parsing replaced by the shared clap
// grammar in `audiohax::cli`. Same library logic runs unchanged below.
use std::fs;
use std::io::Write;

use audiohax::cli::parse_make_packetized;
use audiohax::modem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared clap grammar (S9 §3.7).
    let cli = parse_make_packetized();
    let input_path = cli.input.clone();
    let out_path = cli.out_packetized.to_string_lossy().to_string();
    let out_path = out_path.as_str();

    let compress = cli.compress;
    let rs_data: Option<usize> = cli.rs_data;
    let rs_parity: Option<usize> = cli.rs_parity;
    let rs_shard_size: usize = cli.rs_shard_size;
    let pkt_size: usize = cli.pkt_size;
    let repeats: usize = cli.repeats;

    let payload = fs::read(&input_path)?;
    let filename = input_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("payload");

    // Build frame header+payload (same as modem_encode)
    let frame = modem::build_frame(
        filename, &payload, compress, None, // no encryption here; add CLI option if you want
    )?;

    println!(
        "Built frame: {} bytes (payload {})",
        frame.len(),
        payload.len()
    );

    let packetized = if let (Some(d), Some(p)) = (rs_data, rs_parity) {
        println!(
            "Packetizing with RS: data={} parity={} shard_size={}",
            d, p, rs_shard_size
        );
        modem::packetize_stream_rs(&frame, rs_shard_size, d, p)?
    } else {
        println!(
            "Packetizing with repeats: pkt_size={} repeats={}",
            pkt_size, repeats
        );
        modem::packetize_stream(&frame, pkt_size, repeats)
    };

    let mut f = std::fs::File::create(out_path)?;
    f.write_all(&packetized)?;
    println!("Wrote {} bytes to {}", packetized.len(), out_path);
    Ok(())
}

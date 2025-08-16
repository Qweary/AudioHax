// src/bin/channel_sim.rs
//
// Simple simulator harness:
//  - Builds a frame from input file (uses modem::build_frame)
//  - Packetizes with RS interleaving (modem::packetize_stream_rs_interleaved)
//  - Repeatedly simulates channel corruption using modem::simulate_channel_bytes
//  - Attempts RS depacketize (modem::depacketize_stream_rs) on the corrupted bytes
//  - Records success/failure to CSV
//
// Usage:
//   cargo run --bin channel_sim -- <input_file> --rs-data D --rs-parity P --rs-shard-size S [options]
//
// Options:
//   --trials N                 number of trials per param combo (default 100)
//   --burst-prob-start F       burst start prob (default 0.000)
//   --burst-prob-end F         burst end prob (default 0.010)
//   --burst-steps N            number of burst prob values between start/end (default 6)
//   --flip-prob-start F        random flip start prob (default 0.0)
//   --flip-prob-end F          random flip end prob (default 0.01)
//   --flip-steps N             number of flip prob values between start/end (default 6)
//   --avg-burst-len N          average burst length in bytes (default 128)
//   --out-csv PATH             output CSV path (default: sim_results.csv)
//   --no-interleave            use plain packetize_stream_rs (no interleave)
//   --compress                 compress payload before RS (default off)
//
// Example:
//   cargo run --bin channel_sim -- test.jpg --rs-data 4 --rs-parity 5 --rs-shard-size 128 --trials 200
//

use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use audiohax::modem;
use audiohax::modem::ModemParams;

fn parse_f64(s: &str, default: f64) -> f64 {
    s.parse().ok().unwrap_or(default)
}
fn parse_usize(s: &str, default: usize) -> usize {
    s.parse().ok().unwrap_or(default)
}

fn frange(start: f64, end: f64, steps: usize) -> Vec<f64> {
    if steps <= 1 {
        return vec![start];
    }
    let mut v = Vec::with_capacity(steps);
    for i in 0..steps {
        let t = (i as f64) / ((steps - 1) as f64);
        v.push(start + (end - start) * t);
    }
    v
}

fn print_usage(bin: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <input_file> --rs-data D --rs-parity P --rs-shard-size S [options]", bin);
    eprintln!("Options (defaults shown):");
    eprintln!("  --trials N (100)");
    eprintln!("  --burst-prob-start F (0.0)");
    eprintln!("  --burst-prob-end F (0.01)");
    eprintln!("  --burst-steps N (6)");
    eprintln!("  --flip-prob-start F (0.0)");
    eprintln!("  --flip-prob-end F (0.01)");
    eprintln!("  --flip-steps N (6)");
    eprintln!("  --avg-burst-len N (128)");
    eprintln!("  --out-csv PATH (sim_results.csv)");
    eprintln!("  --no-interleave (use plain RS serialization)");
    eprintln!("  --compress (compress payload before RS)");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let input_path = args[1].clone();

    // defaults
    let mut rs_data: Option<usize> = None;
    let mut rs_parity: Option<usize> = None;
    let mut rs_shard_size: Option<usize> = None;

    let mut trials: usize = 100;
    let mut burst_start: f64 = 0.0;
    let mut burst_end: f64 = 0.01;
    let mut burst_steps: usize = 6;
    let mut flip_start: f64 = 0.0;
    let mut flip_end: f64 = 0.01;
    let mut flip_steps: usize = 6;
    let mut avg_burst_len: usize = 128;
    let mut out_csv = "sim_results.csv".to_string();
    let mut interleave = true;
    let mut compress = false;

    // parse args
    let mut i = 2usize;
    while i < args.len() {
        match args[i].as_str() {
            "--rs-data" => { rs_data = args.get(i+1).and_then(|s| s.parse().ok()); i += 2; }
            "--rs-parity" => { rs_parity = args.get(i+1).and_then(|s| s.parse().ok()); i += 2; }
            "--rs-shard-size" => { rs_shard_size = args.get(i+1).and_then(|s| s.parse().ok()); i += 2; }
            "--trials" => { trials = args.get(i+1).and_then(|s| s.parse().ok()).unwrap_or(trials); i += 2; }
            "--burst-prob-start" => { burst_start = parse_f64(&args[i+1], burst_start); i += 2; }
            "--burst-prob-end" => { burst_end = parse_f64(&args[i+1], burst_end); i += 2; }
            "--burst-steps" => { burst_steps = parse_usize(&args[i+1], burst_steps); i += 2; }
            "--flip-prob-start" => { flip_start = parse_f64(&args[i+1], flip_start); i += 2; }
            "--flip-prob-end" => { flip_end = parse_f64(&args[i+1], flip_end); i += 2; }
            "--flip-steps" => { flip_steps = parse_usize(&args[i+1], flip_steps); i += 2; }
            "--avg-burst-len" => { avg_burst_len = parse_usize(&args[i+1], avg_burst_len); i += 2; }
            "--out-csv" => { out_csv = args.get(i+1).cloned().unwrap_or(out_csv); i += 2; }
            "--no-interleave" => { interleave = false; i += 1; }
            "--compress" => { compress = true; i += 1; }
            _ => { eprintln!("Unknown arg: {}", args[i]); print_usage(&args[0]); std::process::exit(1); }
        }
    }

    if rs_data.is_none() || rs_parity.is_none() || rs_shard_size.is_none() {
        eprintln!("RS parameters are required: --rs-data, --rs-parity, --rs-shard-size");
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let d = rs_data.unwrap();
    let p = rs_parity.unwrap();
    let s = rs_shard_size.unwrap();

    println!("Simulator harness:");
    println!(" input: {}", input_path);
    println!(" RS data/parity/shard_size = {}/{}/{}", d, p, s);
    println!(" trials per combo = {}", trials);
    println!(" burst prob range = {} -> {} (steps {})", burst_start, burst_end, burst_steps);
    println!(" flip prob range = {} -> {} (steps {})", flip_start, flip_end, flip_steps);
    println!(" avg burst len = {}", avg_burst_len);
    println!(" interleave = {}", interleave);
    println!(" compress frame = {}", compress);
    println!(" output csv = {}", out_csv);

    // read payload
    let payload = fs::read(&input_path)?;
    let filename = Path::new(&input_path).file_name().and_then(|s| s.to_str()).unwrap_or("payload");

    // build frame
    let frame = modem::build_frame(filename, &payload, compress, None)?;
    println!("Built frame {} bytes", frame.len());

    // packetize with RS (interleaved or not)
    let serialized = if interleave {
        modem::packetize_stream_rs_interleaved(&frame, d, p, s)
    } else {
        // packetize_stream_rs returns Result<Vec<u8>,_>
        match modem::packetize_stream_rs(&frame, s, d, p) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("packetize_stream_rs failed: {}", e);
                std::process::exit(1);
            }
        }
    };

    println!("Serialized RS bytes = {} bytes", serialized.len());

    // prepare sweep arrays
    let burst_values = frange(burst_start, burst_end, burst_steps);
    let flip_values = frange(flip_start, flip_end, flip_steps);

    // CSV file
    let mut f = fs::File::create(&out_csv)?;
    writeln!(f, "burst_prob,flip_prob,trial,success,recovered_len,expected_len")?;

    // run sweep
    let mut total_runs = 0usize;
    for &bp in &burst_values {
        for &fp in &flip_values {
            println!("Testing burst_prob={:.6}, flip_prob={:.6} ...", bp, fp);
            for tnum in 0..trials {
                total_runs += 1;
                // simulate
                let sim_bytes = modem::simulate_channel_bytes(&serialized, fp, bp, avg_burst_len);
                // attempt RS depacketize
                match modem::depacketize_stream_rs(&sim_bytes) {
                    Ok(recovered) => {
                        let success = if recovered == frame { 1 } else { 0 };
                        writeln!(f, "{:.6},{:.6},{},{},{},{}", bp, fp, tnum, success, recovered.len(), frame.len())?;
                    }
                    Err(_e) => {
                        // failed to reconstruct
                        writeln!(f, "{:.6},{:.6},{},{},{},{}", bp, fp, tnum, 0, 0, frame.len())?;
                    }
                }
                // flush occasionally
                if tnum % 50 == 0 {
                    f.flush().ok();
                }
            }
            println!("  done.");
        }
    }

    println!("Finished {} runs. Results written to {}", total_runs, out_csv);
    Ok(())
}

// tests/modem_cli_roundtrip.rs
//
// AudioHax MFSK modem — BIN-LEVEL CLI round-trip integration net (WS-2, S8 item a).
//
// Where tests/modem_roundtrip.rs and tests/modem_realair.rs exercise the modem
// LIBRARY in-memory, this net drives the actual shipped CLI binaries
// (`modem_encode`, `modem_decode`, `channel_sim`) end to end via
// std::process::Command. It is the guard that the S7 real-air path — chirp sync,
// in-band `CDG1` coding profiles (rs-*/rep/auto) — is correctly WIRED THROUGH THE
// CLI FLAGS the operator actually types, and that the legacy (pilot / header-less)
// invocation stays byte-identical to today.
//
// Discipline:
//   * The assertion is always byte-level DATA IDENTITY (input file bytes ==
//     recovered `*_recovered.bin` bytes), or byte-identity of two WAV files —
//     never "the process didn't panic".
//   * All I/O lands in a per-test unique subdir of the system temp dir; nothing is
//     written into the repo tree. Each test cleans up on success; unique names keep
//     concurrent tests from colliding.
//   * Payloads are ~200-400 bytes of deterministic, non-image-magic bytes (so the
//     decoder's content sniffer always writes `*_recovered.bin`, never .png/.jpg).
//
// Bins are located via Cargo's `CARGO_BIN_EXE_<name>` env vars (set for integration
// tests), so the net follows the build without hard-coded target/debug paths.
//
// Run headless:  cargo test --test modem_cli_roundtrip

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

// ── bin locations (Cargo-provided; see file header) ─────────────────────────
const BIN_ENCODE: &str = env!("CARGO_BIN_EXE_modem_encode");
const BIN_DECODE: &str = env!("CARGO_BIN_EXE_modem_decode");
const BIN_CHANNEL_SIM: &str = env!("CARGO_BIN_EXE_channel_sim");

static UNIQ: AtomicU64 = AtomicU64::new(0);

/// A fresh, unique working directory under the SYSTEM temp dir for one test.
/// Never inside the repo tree. Name folds in the test label, the process id, and a
/// per-process atomic counter so parallel tests never collide.
fn work_dir(label: &str) -> PathBuf {
    let n = UNIQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "audiohax_cli_rt_{}_{}_{}",
        label,
        std::process::id(),
        n
    ));
    std::fs::create_dir_all(&dir).expect("create per-test temp dir");
    dir
}

/// Deterministic, reproducible payload of `len` bytes whose FIRST byte is not any
/// image magic (PNG 0x89 / JPEG 0xFF / GIF 'G' / BMP 0x42), so `modem_decode`'s
/// content sniffer always writes a `*_recovered.bin`.
fn payload(len: usize) -> Vec<u8> {
    let v: Vec<u8> = (0..len).map(|i| ((i * 37 + 11) & 0xFF) as u8).collect();
    debug_assert!(v[0] != 0x89 && v[0] != 0xFF && v[0] != b'G' && v[0] != 0x42);
    v
}

/// Run a bin with args; assert exit-success and, on failure, panic with the full
/// captured stdout+stderr so a red test is debuggable from the failure message.
fn run(bin: &str, args: &[&str]) -> Output {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));
    assert!(
        out.status.success(),
        "process failed: {bin} {args:?}\n--- exit: {:?} ---\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    out
}

/// Path a `modem_decode` recovered-file lands at for a given output basename, given
/// a non-magic payload (always the `.bin` extension).
fn recovered_bin(base: &Path) -> PathBuf {
    let mut s = base.as_os_str().to_owned();
    s.push("_recovered.bin");
    PathBuf::from(s)
}

/// Assert two byte slices are equal, with a compact, debuggable diff summary.
fn assert_bytes_eq(expected: &[u8], got: &[u8], ctx: &str) {
    assert_eq!(
        expected.len(),
        got.len(),
        "{ctx}: length mismatch (expected {} bytes, recovered {})",
        expected.len(),
        got.len()
    );
    if let Some(i) = expected.iter().zip(got).position(|(a, b)| a != b) {
        panic!(
            "{ctx}: first byte mismatch at offset {i} (expected {:#04x}, got {:#04x})",
            expected[i], got[i]
        );
    }
}

// ============================================================================
// TEST 1 — chirp sync + Reed-Solomon coding profile, CDG1 auto-detected on decode.
// Property: `modem_encode --sync-mode chirp --coding-profile rs-medium` then
// `modem_decode --sync-mode chirp` (NO profile flag) recovers the input byte-for-
// byte. Pins that the chirp start-of-burst path AND the in-band CDG1 rate header
// are wired through the CLI: the decoder learns the rate from the stream, needing
// only the matching --sync-mode.
// ============================================================================
#[test]
fn cli_chirp_rs_medium_roundtrip_byte_exact() {
    let dir = work_dir("chirp_rs");
    let inp = dir.join("in.bin");
    let wav = dir.join("out.wav");
    let base = dir.join("rec");
    let data = payload(320);
    std::fs::write(&inp, &data).unwrap();

    run(
        BIN_ENCODE,
        &[
            wav.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--sync-mode",
            "chirp",
            "--coding-profile",
            "rs-medium",
        ],
    );
    let dec = run(
        BIN_DECODE,
        &[
            wav.to_str().unwrap(),
            base.to_str().unwrap(),
            "--sync-mode",
            "chirp",
        ],
    );
    // Sanity: the decode log should announce the auto-detected in-band rate header.
    let log = String::from_utf8_lossy(&dec.stdout);
    assert!(
        log.contains("CDG1") && log.contains("RsRate(Medium)"),
        "expected decode to auto-detect the CDG1 RsRate(Medium) header; decode stdout was:\n{log}"
    );

    let got = std::fs::read(recovered_bin(&base)).expect("recovered .bin must exist");
    assert_bytes_eq(&data, &got, "chirp+rs-medium CLI round-trip");
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// TEST 2 — legacy header-less repetition stream still decodes (regression guard).
// Property: pure-legacy encode (default pilot sync, --coding-profile legacy,
// --repeats 3) then `modem_decode --repeats 3` (no sync-mode) recovers byte-exact.
// This is the "the legacy path is unbroken by the S7 flags" guard.
// ============================================================================
#[test]
fn cli_legacy_repetition_stream_still_decodes() {
    let dir = work_dir("legacy_rep");
    let inp = dir.join("in.bin");
    let wav = dir.join("out.wav");
    let base = dir.join("rec");
    let data = payload(240);
    std::fs::write(&inp, &data).unwrap();

    run(
        BIN_ENCODE,
        &[
            wav.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--coding-profile",
            "legacy",
            "--repeats",
            "3",
        ],
    );
    run(
        BIN_DECODE,
        &[
            wav.to_str().unwrap(),
            base.to_str().unwrap(),
            "--repeats",
            "3",
        ],
    );

    let got = std::fs::read(recovered_bin(&base)).expect("recovered .bin must exist");
    assert_bytes_eq(&data, &got, "legacy repetition CLI round-trip");
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// TEST 3 — the new S7 flags' DEFAULTS reproduce today's output byte-for-byte.
// Property: encoding the SAME input once with no new flags and once with explicit
// `--sync-mode pilot --coding-profile legacy` yields BYTE-IDENTICAL WAV files.
// Pins that `pilot` + `legacy` are true no-ops relative to the pre-S8 output.
// ============================================================================
#[test]
fn cli_default_flags_are_byte_identical_wav() {
    let dir = work_dir("default_identity");
    let inp = dir.join("in.bin");
    let wav_a = dir.join("a.wav");
    let wav_b = dir.join("b.wav");
    let data = payload(300);
    std::fs::write(&inp, &data).unwrap();

    run(
        BIN_ENCODE,
        &[wav_a.to_str().unwrap(), inp.to_str().unwrap()],
    );
    run(
        BIN_ENCODE,
        &[
            wav_b.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--sync-mode",
            "pilot",
            "--coding-profile",
            "legacy",
        ],
    );

    let a = std::fs::read(&wav_a).unwrap();
    let b = std::fs::read(&wav_b).unwrap();
    assert_bytes_eq(&a, &b, "default vs explicit-pilot/legacy WAV identity");
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// TEST 4 — chirp sync + repetition coding profile (CDG1) round-trip.
// Property: `modem_encode --sync-mode chirp --coding-profile rep --repeats 3` then
// `modem_decode --sync-mode chirp` (no profile flag) recovers byte-exact. Exercises
// the chirp path with the header-bearing REPETITION profile (distinct from the RS
// profile in test 1).
// ============================================================================
#[test]
fn cli_chirp_rep_profile_roundtrip_byte_exact() {
    let dir = work_dir("chirp_rep");
    let inp = dir.join("in.bin");
    let wav = dir.join("out.wav");
    let base = dir.join("rec");
    let data = payload(256);
    std::fs::write(&inp, &data).unwrap();

    run(
        BIN_ENCODE,
        &[
            wav.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--sync-mode",
            "chirp",
            "--coding-profile",
            "rep",
            "--repeats",
            "3",
        ],
    );
    run(
        BIN_DECODE,
        &[
            wav.to_str().unwrap(),
            base.to_str().unwrap(),
            "--sync-mode",
            "chirp",
        ],
    );

    let got = std::fs::read(recovered_bin(&base)).expect("recovered .bin must exist");
    assert_bytes_eq(&data, &got, "chirp+rep CLI round-trip");
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// TEST 6 — auto coding profile selects an RS rate from --snr-db and round-trips.
// Property: `modem_encode --coding-profile auto --snr-db 25 --sync-mode chirp` then
// `modem_decode --sync-mode chirp` recovers byte-exact. select_rate thresholds put
// 25 dB in the High band, so we ALSO assert the decode's CDG1 log names RsRate(High)
// — proving the auto path is wired AND selected the expected rate.
// ============================================================================
#[test]
fn cli_auto_profile_selects_by_snr_and_roundtrips() {
    let dir = work_dir("auto_snr");
    let inp = dir.join("in.bin");
    let wav = dir.join("out.wav");
    let base = dir.join("rec");
    let data = payload(300);
    std::fs::write(&inp, &data).unwrap();

    run(
        BIN_ENCODE,
        &[
            wav.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--coding-profile",
            "auto",
            "--snr-db",
            "25",
            "--sync-mode",
            "chirp",
        ],
    );
    let dec = run(
        BIN_DECODE,
        &[
            wav.to_str().unwrap(),
            base.to_str().unwrap(),
            "--sync-mode",
            "chirp",
        ],
    );
    let log = String::from_utf8_lossy(&dec.stdout);
    assert!(
        log.contains("RsRate(High)"),
        "auto @ 25 dB must select RsRate::High (>20 dB threshold); decode stdout was:\n{log}"
    );

    let got = std::fs::read(recovered_bin(&base)).expect("recovered .bin must exist");
    assert_bytes_eq(&data, &got, "auto-profile (snr=25) CLI round-trip");
    let _ = std::fs::remove_dir_all(&dir);
}

// ============================================================================
// TEST 5 — full CLI acoustic-channel E2E: chirp + RS survives a mild acoustic link.
//
// Property: `modem_encode --sync-mode chirp --coding-profile rs-medium` → push the
// rendered audio through the S7 seeded acoustic channel via the `channel_sim` bin
// (`--mode acoustic`, a NON-TRIVIAL but in-envelope impairment: start offset + clock
// drift + carrier freq offset + a short multipath echo) → `modem_decode --sync-mode
// chirp` → recovers the payload byte-exact. This is the end-to-end acoustic guard:
// chirp start-of-burst detection + drift-tracking timing recovery + RS FEC together
// carry the data through a real audio-domain channel driven entirely from the CLI.
//
// `channel_sim --mode acoustic` reads/writes RAW little-endian i16 samples (NOT a
// WAV container), so this test bridges the container boundary with `hound`: read the
// encoder's WAV → dump raw i16 → run channel_sim → re-wrap the perturbed samples in
// a WAV at the same sample rate → decode. The channel is seeded (deterministic).
//
// Channel params were chosen inside the S7 recovery envelope (cf. the recovered
// freq-offset+multipath and clock-drift cases in tests/modem_realair.rs). If a
// future change tightens the envelope below these values this test flips red — that
// is the intended signal, not flakiness (the channel is deterministic via
// --acoustic-seed).
// ============================================================================
#[test]
fn cli_acoustic_channel_chirp_rs_e2e_byte_exact() {
    let dir = work_dir("acoustic_e2e");
    let inp = dir.join("in.bin");
    let wav_tx = dir.join("tx.wav");
    let raw_tx = dir.join("tx.raw");
    let raw_rx = dir.join("rx.raw");
    let wav_rx = dir.join("rx.wav");
    let base = dir.join("rec");
    let data = payload(300);
    std::fs::write(&inp, &data).unwrap();

    // 1) encode chirp + rs-medium.
    run(
        BIN_ENCODE,
        &[
            wav_tx.to_str().unwrap(),
            inp.to_str().unwrap(),
            "--sync-mode",
            "chirp",
            "--coding-profile",
            "rs-medium",
        ],
    );

    // 2) WAV -> raw i16 LE (channel_sim acoustic mode consumes raw i16 samples).
    let reader = hound::WavReader::open(&wav_tx).expect("open tx wav");
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .collect::<Result<_, _>>()
        .expect("read i16 samples");
    let mut raw = Vec::with_capacity(samples.len() * 2);
    for s in &samples {
        raw.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(&raw_tx, &raw).unwrap();

    // 3) acoustic channel — non-trivial but inside the S7 recovery envelope, seeded.
    run(
        BIN_CHANNEL_SIM,
        &[
            raw_tx.to_str().unwrap(),
            raw_rx.to_str().unwrap(),
            "--mode",
            "acoustic",
            "--acoustic-seed",
            "42",
            "--start-offset",
            "200",
            "--clock-ppm",
            "300",
            "--freq-offset",
            "8",
            "--echo-delay",
            "96",
            "--echo-gain",
            "0.3",
        ],
    );

    // 4) raw i16 LE -> WAV (same sample rate / mono / 16-bit as the encoder emitted).
    let rx_bytes = std::fs::read(&raw_rx).unwrap();
    assert!(
        rx_bytes.len() % 2 == 0,
        "channel_sim acoustic output must be an even number of bytes (raw i16 LE)"
    );
    let rx_samples: Vec<i16> = rx_bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut writer = hound::WavWriter::create(&wav_rx, spec).expect("create rx wav");
    for s in rx_samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();

    // 5) decode chirp; RS-medium + chirp sync must recover the payload byte-exact.
    run(
        BIN_DECODE,
        &[
            wav_rx.to_str().unwrap(),
            base.to_str().unwrap(),
            "--sync-mode",
            "chirp",
        ],
    );

    let got = std::fs::read(recovered_bin(&base)).expect("recovered .bin must exist");
    assert_bytes_eq(
        &data,
        &got,
        "chirp+rs-medium through acoustic channel (CLI E2E)",
    );
    let _ = std::fs::remove_dir_all(&dir);
}

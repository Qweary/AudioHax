//! tests/render_golden_s122.rs — S122 C-2: the in-repo guard that FAILS when the rendered audio
//! changes.
//!
//! WHY THIS FILE EXISTS. After Phase 0 the CLI is a thin adapter over `audiohax::render::render`.
//! Every pre-existing "identity" test compares the shipped binary with ITSELF (two runs) or the
//! CLI with the lib (now the same code). A change INSIDE `render()` moves both sides equally, so
//! it ships green: the S122 Quality Gate moved the non-loop tail 1 500 → 1 510 ms and all 716
//! tests passed (`s122-06-phase0-seal.md` § 6.1, mutation M8). This file compares `render()`
//! against values recorded from the PRE-Phase-0 binary (HEAD `1d5c4db`), so it pins yesterday's
//! sound, not whatever today's tree produces.
//!
//! TWO GOLDENS, BECAUSE THEY ARE PORTABLE TO DIFFERENT DEGREES.
//!
//!   1. STRUCTURE golden (`golden_structure_portable`, runs on EVERY platform). The SHA-256 of a
//!      canonical, integer-only serialization (`audiohax-structure-v1`, see [`structure_bytes`])
//!      of: sample rate, `total_steps`, `grid_ms_per_step`, `instrument_count`, per-voice
//!      programs, every timeline event (`at_ms`, kind, channel, note/program, velocity), and the
//!      final interleaved sample count. No float is hashed. It catches:
//!        * any COMPOSITION change: a note, an onset, a duration, a velocity, a program, the seed
//!          (QG mutation M12, `--seed N` composing as `N+1`);
//!        * any LENGTH change: the synthesis tail or the loop fold (M8 moves the sample count by
//!          882 = 10 ms × 44.1 kHz × 2 channels). The event list alone CANNOT see M8: the tail
//!          is a synthesis parameter, not an event.
//!
//!      Cases use an EXPLICIT seed only. The image-derived seed (S91, `composition::image_seed`)
//!      hashes the raw f32 BITS of 11 image features, some computed through platform libm
//!      (`sin`/`cos`/`atan2`/`ln` in `pure_analysis.rs`), so a 1-ULP libm difference changes
//!      the whole piece. That case lives in the platform-gated golden below.
//!
//!   2. WAV-BYTES golden (`golden_wav_bytes_x86_64_linux`). The SHA-256 of the exact `.wav` file
//!      `synth_sink::write_stereo_wav` writes (compare with `sha256sum` on a CLI render). It
//!      catches everything above PLUS synthesis-only changes that keep the length: reverb /
//!      chorus, gain or soft-clip, sample quantization, the WAV header, a different SoundFont.
//!      It is platform-bound: s121-01 § 3.3 measured identical composition but ±1 LSB samples
//!      between native and wasm on a PNG input (float rounding in synthesis). So it runs ONLY on
//!      x86_64 Linux (CI's `ubuntu-latest` job and the operator's box). Elsewhere it is reported
//!      as `ignored` with the reason, and it also returns early if forced with `--ignored`.
//!      It also requires the embedded SoundFont to be the one the goldens were taken with
//!      (`SF2_SHA256`): `ci.yml` fetches GeneralUser-GS from an UNPINNED upstream `main`, so an
//!      upstream font update would otherwise be a red that no repo change caused. On a font
//!      mismatch the WAV check is SKIPPED with a printed reason (the structure golden, which is
//!      font-independent, still runs).
//!
//! WHICH GOLDEN CATCHES WHICH CLASS (each row demonstrated by a mutation in `s122-05` Wave 3):
//!
//!   | change                                         | structure | WAV (x86_64 Linux) |
//!   |------------------------------------------------|-----------|--------------------|
//!   | composition (notes / timing / seed / programs) | FAILS     | FAILS              |
//!   | render length (tail, loop fold)                | FAILS     | FAILS              |
//!   | synthesis only, same length (reverb, gain, …)  | passes    | FAILS              |
//!
//! PROVENANCE. Every value in `GOLDENS` was produced by HEAD `1d5c4db` (pre-Phase-0):
//! `wav_sha256` = `sha256sum` of the WAV that the BEFORE release binary
//! (`audiohax-before-1d5c4db`, sha256 `4cf8245e…2dd3`) wrote for the same arguments;
//! `structure_sha256` = the same serialization emitted by an instrumented build of the
//! `1d5c4db` source. The working tree reproduces all of them (see `s122-05` Wave 3).
//!
//! UPDATING THE GOLDEN (an INTENTIONAL sound change). On a mismatch the failing test prints, for
//! each case, the exact replacement `Golden { … },` line computed from the current tree.
//!   1. Confirm the change is intended (listen to it; this guard cannot judge taste).
//!   2. Run `cargo test --test render_golden_s122` on x86_64 Linux so both hashes are printed.
//!   3. Paste the printed line(s) over the matching line(s) in `GOLDENS`, in the SAME commit as
//!      the sound change, and say in the commit message which cases moved and why.
//!
//!   Never update a golden to make an UNEXPLAINED red go away: that red is this file working.
//!   Off x86_64 Linux the printed line carries the OLD `wav_sha256` (it cannot be computed
//!   there); regenerate it on x86_64 Linux before committing.
//!
//! Fixture: `assets/images/Lena.png`, the only committed PNG (PNG, not JPEG, because JPEG decode
//! differs across CPUs: s121-01 § 7 U1). `--steps 6` scans 6 bars; the piece is still 36 steps,
//! about 19.5 s. Cost: four debug renders on four threads, about 6–8 s wall on an 8-core box.
//!
//! Filesystem: the WAV golden writes each case's WAV to `std::env::temp_dir()` through the lib's
//! own writer and removes it, because the property pinned IS the bytes that writer puts on disk.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use audiohax::cli::{PipelineArgs, Toggle, V2Spec};
use audiohax::mapping_loader::load_mappings;
use audiohax::render::{render, ImageInput, RenderOutput, RenderRequest};
use audiohax::synth_sink::{write_stereo_wav, MidiCmd, SoundFontSource};

// ─────────────────────────────────────────────────────────────────────────────
// The goldens. ONE LINE PER CASE. Update procedure: see the module doc.
// ─────────────────────────────────────────────────────────────────────────────

/// One pinned render. `frames` is the interleaved sample count (`RenderOutput.interleaved.len()`)
/// and `events` the timeline length; both are inside `structure_sha256` too and are kept
/// separately only so a failure says WHAT moved.
struct Golden {
    case: &'static str,
    frames: usize,
    events: usize,
    structure_sha256: &'static str,
    wav_sha256: &'static str,
}

#[rustfmt::skip]
const GOLDENS: &[Golden] = &[
    Golden { case: "lena_seed7", frames: 1719900, events: 352, structure_sha256: "2ac594b0ccb0f9da703336aaf49c87fd2874ce3bf96344b95aa48a26340220f5", wav_sha256: "4f3b3fd8f688f25fd64304d954155ecc5c52c735b5c89b6f5df86f5e6b76b819" },
    Golden { case: "lena_seed7_expr_v2all", frames: 1719900, events: 352, structure_sha256: "c866876bdb6b598f9169c5662c90a8888d86093b0a39cd11df3d2e8e735b23fc", wav_sha256: "35568093fc4d2515f5a6f04d8494ad688af5f722fae890ae473b6e8d194d61da" },
    Golden { case: "lena_seed7_loop", frames: 1587600, events: 352, structure_sha256: "89a35f428b1be2dca7dcefed5b9368e77ee12b9326af3295279fc16f600e0c62", wav_sha256: "723e2554e39e30d9ad664bcace3d5b0c630ae3e6551916985b349467e8639441" },
    Golden { case: "lena_image_seed", frames: 1719900, events: 350, structure_sha256: "7b68d9c885de91dc766174b275dafe6156059e38fafc26ad0b4601c362a3825d", wav_sha256: "4a55cb9539b5e63a0b93b9f554387efd1a452687711b2ee7bed02a3252e122ab" },
];

/// SHA-256 of the GeneralUser-GS SoundFont the WAV goldens were rendered with
/// (`assets/soundfonts/default.sf2`, gitignored, embedded by `include_bytes!`). It equals the
/// file `ci.yml` fetched on 2026-10-20.
const SF2_SHA256: &str = "9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe";

/// Whether this case's composition depends on the image-derived seed (f32 bits through libm),
/// which keeps it out of the portable golden. See the module doc.
fn portable(case: &str) -> bool {
    case != "lena_image_seed"
}

// ─────────────────────────────────────────────────────────────────────────────
// Cases: the CLI arguments each golden was recorded with, as a `RenderRequest`.
// ─────────────────────────────────────────────────────────────────────────────

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// `render assets/images/Lena.png --wav X <args>`, for the args of `case`.
fn request(case: &str) -> RenderRequest {
    let base = RenderRequest {
        seed: Some(7),
        loop_cue: false,
        expression: Toggle::Off,
        v2: None,
        pipeline: PipelineArgs {
            steps: 6,
            ..PipelineArgs::default()
        },
        reverb: Toggle::On,
        gain: 1.0,
        event_dump: None,
    };
    match case {
        // --seed 7 --steps 6
        "lena_seed7" => base,
        // --seed 7 --steps 6 --expression on --v2 all
        "lena_seed7_expr_v2all" => RenderRequest {
            expression: Toggle::On,
            v2: Some("all".parse::<V2Spec>().expect("v2 spec parses")),
            ..base
        },
        // --seed 7 --steps 6 --loop
        "lena_seed7_loop" => RenderRequest {
            loop_cue: true,
            ..base
        },
        // --steps 6  (no --seed: the S91 image-derived seed)
        "lena_image_seed" => RenderRequest { seed: None, ..base },
        other => panic!("no request for golden case {other}"),
    }
}

/// Every case rendered once per test binary, each on its own new thread (the seed / expression /
/// cadence registers are thread-local, so the renders cannot see each other; the event dump is
/// process-global and is never armed here). Order follows `GOLDENS`.
fn renders() -> &'static [(&'static str, RenderOutput)] {
    static R: OnceLock<Vec<(&'static str, RenderOutput)>> = OnceLock::new();
    R.get_or_init(|| {
        let handles: Vec<_> = GOLDENS
            .iter()
            .map(|g| {
                let case = g.case;
                std::thread::spawn(move || {
                    let mappings =
                        load_mappings(manifest("assets/mappings.json").to_str().expect("utf-8"))
                            .expect("assets/mappings.json loads");
                    let lena = manifest("assets/images/Lena.png");
                    let out = render(
                        &request(case),
                        ImageInput::Path(&lena),
                        &mappings,
                        SoundFontSource::Bundled,
                        &mut |_| Ok(()),
                    )
                    .unwrap_or_else(|e| panic!("golden case {case}: render failed: {e:?}"));
                    (case, out)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("render thread panicked"))
            .collect()
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Canonical serialization + hashing
// ─────────────────────────────────────────────────────────────────────────────

/// `audiohax-structure-v1`: integers only, little-endian, in this order. The BEFORE values were
/// produced by the SAME layout emitted from `1d5c4db`'s `main.rs` (s122-05 Wave 3). Changing this
/// layout changes every `structure_sha256`; bump the tag and re-record if you ever must.
fn structure_bytes(out: &RenderOutput) -> Vec<u8> {
    let s = &out.summary;
    let mut c = Vec::with_capacity(64 + out.events.len() * 12);
    c.extend_from_slice(b"audiohax-structure-v1\0");
    c.extend_from_slice(&out.sample_rate.to_le_bytes());
    c.extend_from_slice(&(s.total_steps as u64).to_le_bytes());
    c.extend_from_slice(&s.grid_ms_per_step.to_le_bytes());
    c.extend_from_slice(&(s.instrument_count as u64).to_le_bytes());
    c.extend_from_slice(&(s.programs.len() as u64).to_le_bytes());
    c.extend_from_slice(&s.programs);
    c.extend_from_slice(&(out.events.len() as u64).to_le_bytes());
    for e in &out.events {
        c.extend_from_slice(&e.at_ms.to_le_bytes());
        let (tag, ch, a, b) = match e.cmd {
            MidiCmd::NoteOn {
                channel,
                note,
                velocity,
            } => (1u8, channel, note, velocity),
            MidiCmd::NoteOff { channel, note } => (2u8, channel, note, 0u8),
            MidiCmd::ProgramChange { channel, program } => (3u8, channel, program, 0u8),
        };
        c.extend_from_slice(&[tag, ch, a, b]);
    }
    c.extend_from_slice(&(out.interleaved.len() as u64).to_le_bytes());
    c
}

/// The exact bytes `write_stereo_wav` puts on disk for this render (the file is removed).
fn wav_bytes(case: &str, out: &RenderOutput) -> Vec<u8> {
    // Unique per call: both tests may write the same case at the same time.
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "audiohax-render-golden-s122-{}-{}-{case}.wav",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    write_stereo_wav(&path, out.sample_rate, &out.interleaved)
        .unwrap_or_else(|e| panic!("golden case {case}: write_stereo_wav failed: {e:?}"));
    let bytes = std::fs::read(&path).expect("read back the golden WAV");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// FIPS 180-4 SHA-256, lowercase hex (the crate has no hash dependency and this file may not add
/// one). Checked against the FIPS vectors in `golden_sha256_matches_fips_vectors`, so a value in
/// `GOLDENS` can be reproduced with `sha256sum`.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 64];
    for block in msg.chunks_exact(64) {
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// The paste-ready replacement line for one case.
fn golden_line(case: &str, out: &RenderOutput, structure: &str, wav: &str) -> String {
    format!(
        "    Golden {{ case: \"{case}\", frames: {}, events: {}, structure_sha256: \"{structure}\", wav_sha256: \"{wav}\" }},",
        out.interleaved.len(),
        out.events.len()
    )
}

fn wav_golden_platform() -> bool {
    cfg!(all(target_os = "linux", target_arch = "x86_64"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// The hash itself: FIPS 180-4 vectors (empty, "abc", the 448-bit two-block message). If this
/// fails, every golden below is meaningless, so it is checked separately.
#[test]
fn golden_sha256_matches_fips_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

/// Composition + render length, on every platform: the pre-Phase-0 piece's notes, onsets,
/// velocities, programs and sample count, for every explicit-seed case.
#[test]
fn golden_structure_portable() {
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for (g, (case, out)) in GOLDENS.iter().zip(renders()) {
        assert_eq!(g.case, *case);
        if !portable(case) {
            continue;
        }
        let structure = sha256_hex(&structure_bytes(out));
        let wav = if wav_golden_platform() {
            sha256_hex(&wav_bytes(case, out))
        } else {
            g.wav_sha256.to_string()
        };
        if out.interleaved.len() != g.frames {
            let d = out.interleaved.len() as i64 - g.frames as i64;
            failures.push(format!(
                "{case}: interleaved sample count {} != golden {} (Δ {d} samples = {:.1} ms of stereo audio): the render LENGTH moved (tail / loop fold)",
                out.interleaved.len(),
                g.frames,
                d as f64 / 2.0 / out.sample_rate as f64 * 1000.0
            ));
        }
        if out.events.len() != g.events {
            failures.push(format!(
                "{case}: timeline has {} events != golden {}: the COMPOSITION moved",
                out.events.len(),
                g.events
            ));
        }
        if structure != g.structure_sha256 {
            failures.push(format!(
                "{case}: structure sha256 {structure} != golden {}: notes / onsets / velocities / programs / length moved",
                g.structure_sha256
            ));
        }
        lines.push(golden_line(case, out, &structure, &wav));
    }
    assert!(
        failures.is_empty(),
        "\nRENDERED AUDIO CHANGED (structure golden, tests/render_golden_s122.rs):\n  {}\n\n\
         If, and only if, this change is INTENDED, replace the matching line(s) in GOLDENS with:\n{}\n{}\
         See the module doc's update procedure.\n",
        failures.join("\n  "),
        lines.join("\n"),
        if wav_golden_platform() {
            ""
        } else {
            "(wav_sha256 above is the OLD value: regenerate it on x86_64 Linux.)\n"
        }
    );
}

/// The exact WAV bytes, every case including the image-derived seed. Platform-bound (float
/// rounding in synthesis, libm in the image seed), so x86_64 Linux only.
#[test]
#[cfg_attr(
    not(all(target_os = "linux", target_arch = "x86_64")),
    ignore = "WAV-byte golden is x86_64-Linux-only (synthesis float rounding is platform-bound, s121-01 § 3.3); the structure golden still runs"
)]
fn golden_wav_bytes_x86_64_linux() {
    if !wav_golden_platform() {
        eprintln!(
            "golden_wav_bytes_x86_64_linux: SKIPPED, not x86_64 Linux (forced with --ignored)"
        );
        return;
    }
    let sf2 = std::fs::read(manifest("assets/soundfonts/default.sf2"))
        .expect("assets/soundfonts/default.sf2 (embedded at build time) is readable");
    let sf2_sha = sha256_hex(&sf2);
    if sf2_sha != SF2_SHA256 {
        eprintln!(
            "golden_wav_bytes_x86_64_linux: SKIPPED. The embedded SoundFont sha256 is {sf2_sha}, \
             the goldens were rendered with {SF2_SHA256}. WAV bytes cannot be compared across \
             fonts. The structure golden (font-independent) still ran."
        );
        return;
    }
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for (g, (case, out)) in GOLDENS.iter().zip(renders()) {
        assert_eq!(g.case, *case);
        let structure = sha256_hex(&structure_bytes(out));
        let wav = sha256_hex(&wav_bytes(case, out));
        if wav != g.wav_sha256 {
            failures.push(format!(
                "{case}: WAV sha256 {wav} != golden {} (structure {}): {}",
                g.wav_sha256,
                if structure == g.structure_sha256 {
                    "UNCHANGED"
                } else {
                    "ALSO CHANGED"
                },
                if structure == g.structure_sha256 {
                    "a SYNTHESIS-only change (reverb / gain / quantization / header / font)"
                } else {
                    "the composition or length moved"
                }
            ));
        }
        if !portable(case) && structure != g.structure_sha256 {
            failures.push(format!(
                "{case}: structure sha256 {structure} != golden {} (image-derived seed path)",
                g.structure_sha256
            ));
        }
        lines.push(golden_line(case, out, &structure, &wav));
    }
    assert!(
        failures.is_empty(),
        "\nRENDERED AUDIO CHANGED (WAV golden, tests/render_golden_s122.rs):\n  {}\n\n\
         If, and only if, this change is INTENDED, replace the matching line(s) in GOLDENS with:\n{}\n\
         See the module doc's update procedure.\n",
        failures.join("\n  "),
        lines.join("\n")
    );
}

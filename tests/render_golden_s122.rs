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
//! TWO GOLDENS: ONE GUARDS CI ON EVERY OS, ONE IS A LOCAL OPT-IN GUARD.
//!
//!   1. STRUCTURE golden (`golden_structure_portable`). ALWAYS ON, every OS, every `cargo test`,
//!      including CI. The SHA-256 of a canonical, integer-only serialization
//!      (`audiohax-structure-v1`, see [`structure_bytes`]) of: sample rate, `total_steps`,
//!      `grid_ms_per_step`, `instrument_count`, per-voice programs, every timeline event
//!      (`at_ms`, kind, channel, note/program, velocity), and the final interleaved sample count.
//!      No float is hashed. It catches:
//!        * any COMPOSITION change on an explicit seed: a note, an onset, a duration, a velocity,
//!          a program, the seed itself (QG mutation M12, `--seed N` composing as `N+1`);
//!        * any LENGTH change: the synthesis tail or the loop fold (M8 moves the sample count by
//!          882 = 10 ms × 44.1 kHz × 2 channels). The event list alone CANNOT see M8: the tail
//!          is a synthesis parameter, not an event.
//!
//!      It does not depend on the platform's libm: it stayed green with EVERY libm result moved
//!      by ±1 ULP (an `LD_PRELOAD` perturbation of ~22 M calls, S122 Quality Gate). Cases use an
//!      EXPLICIT seed only, because the image-derived seed (S91, `composition::image_seed`)
//!      hashes the raw f32 BITS of image features computed through libm
//!      (`sin`/`cos`/`atan2`/`ln` in `pure_analysis.rs`).
//!
//!   2. WAV-BYTES golden (`golden_wav_bytes_local_opt_in`). LOCAL and OPT-IN: it is `#[ignore]`d
//!      and it also does nothing unless `AUDIOHAX_WAV_GOLDEN=1` is set, so CI (which runs plain
//!      `cargo test`) never runs it, and neither does a blanket `--include-ignored` without the
//!      variable. It hashes the exact `.wav` file `synth_sink::write_stereo_wav` writes (compare
//!      with `sha256sum` on a CLI render) for every case, and also checks the structure of the
//!      image-derived-seed case. WHY IT IS NOT IN CI: synthesis output is libm-bound. With one
//!      libm result moved by 1 ULP in `log10f`, `powf`, `sinf` or `cosf` alone, every WAV hash
//!      moves (S122 Quality Gate), and the goldens were recorded on glibc 2.43; a CI runner on
//!      another glibc could go red with no change in this repo.
//!
//!      Run it, on the machine the goldens were recorded on (or one whose libm matches):
//!
//!      ```text
//!      AUDIOHAX_WAV_GOLDEN=1 cargo test --test render_golden_s122 -- --include-ignored
//!      ```
//!
//!      It needs the SoundFont the goldens were taken with (`SF2_SHA256`), and it compares a
//!      small libm fingerprint (`LIBM_FINGERPRINT`, results of the libm functions the render
//!      uses on fixed inputs) against the recording machine's. On a WAV red with a fingerprint
//!      mismatch it names the libm functions that differ, so a libm red is not read as a sound
//!      change. The fingerprint is a diagnostic over probe inputs only, not a proof that two
//!      libms agree on the render's inputs.
//!
//! WHICH GUARD CATCHES WHICH CLASS (each row demonstrated by a mutation or a libm perturbation in
//! `s122-05` Waves 3–4):
//!
//!   | change                                                 | CI: structure | LOCAL: WAV (opt-in)     |
//!   |--------------------------------------------------------|---------------|-------------------------|
//!   | composition, explicit seed (notes / timing / programs) | FAILS         | FAILS                   |
//!   | render length (tail, loop fold)                        | FAILS         | FAILS                   |
//!   | synthesis only, same length (reverb, gain, clip, …)    | passes        | FAILS                   |
//!   | image-derived-seed path (S91 `image_seed`)             | passes        | FAILS                   |
//!   | a platform libm 1-ULP difference (not a sound change)  | passes        | FAILS (why it is local) |
//!
//!   So CI guards STRUCTURE (length, events, programs). SOUND-ONLY changes (same notes, same
//!   length, different samples) and the image-derived seed are guarded ONLY locally: run the
//!   opt-in command above before committing any change that could touch synthesis.
//!
//! THE FULL GUARD lives outside this repository: a local before/after harness kept outside the
//! repo, which renders a matrix of CLI configurations with the pre-Phase-0 release binary and
//! the current one and byte-compares every output. This file is the in-repo subset of it.
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
//!   2. Run the opt-in command above, so both hashes are computed and printed.
//!   3. Paste the printed line(s) over the matching line(s) in `GOLDENS`, in the SAME commit as
//!      the sound change, and say in the commit message which cases moved and why.
//!
//!   Never update a golden to make an UNEXPLAINED red go away: that red is this file working.
//!   Never paste WAV values computed on a machine whose libm fingerprint differs: re-record the
//!   whole table and `LIBM_FINGERPRINT` together, on one machine, and say so in the commit.
//!   Without the opt-in variable the printed line carries the OLD `wav_sha256`.
//!
//! Fixture: `assets/images/Lena.png`, the only committed PNG (PNG, not JPEG, because JPEG decode
//! differs across CPUs: s121-01 § 7 U1). `--steps 6` scans 6 bars; the piece is still 36 steps,
//! about 19.5 s. Cost: three debug renders on three threads (four with the opt-in), about 6–8 s
//! wall on an 8-core box.
//!
//! Filesystem: only the opt-in WAV golden writes, each case's WAV to `std::env::temp_dir()`
//! through the lib's own writer, and removes it, because the property pinned IS the bytes that
//! writer puts on disk. The always-on path writes nothing.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::hint::black_box;
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

/// The opt-in switch for the local WAV golden. Only the exact value `1` enables it.
const WAV_GOLDEN_ENV: &str = "AUDIOHAX_WAV_GOLDEN";

/// The libm fingerprint of the machine the WAV goldens were recorded on (x86_64 Linux,
/// glibc 2.43), one entry per probed function, from [`libm_fingerprint`]. Diagnostic only.
#[rustfmt::skip]
const LIBM_FINGERPRINT: &[(&str, &str)] = &[
    ("sinf", "e50e3adff704931c"),
    ("cosf", "198c9474b74a745b"),
    ("powf", "8901514243eed73b"),
    ("log10f", "9f803a95d7e15dd1"),
    ("expf", "b41755ecf98c2b2d"),
    ("exp2f", "6147fdb43d343040"),
    ("tanhf", "8e3e06b9401e0e81"),
    ("atan2f", "2775e464552a883a"),
    ("hypotf", "a76ef1e0026d191c"),
    ("sin", "3c741a3ce2259eaa"),
    ("cos", "161f2b6e31e98be3"),
    ("atan2", "8690f9c9a231cdaf"),
    ("exp", "f363ec20c061b28f"),
    ("log", "d3c49f8b61121b96"),
    ("log10", "c4ab1fcbb9214a3b"),
];

/// Whether this case's composition depends on the image-derived seed (f32 bits through libm),
/// which keeps it out of the always-on golden. See the module doc.
fn portable(case: &str) -> bool {
    case != "lena_image_seed"
}

fn wav_golden_enabled() -> bool {
    std::env::var(WAV_GOLDEN_ENV).is_ok_and(|v| v == "1")
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

/// The cases this process needs rendered, once per test binary, each on its own new thread (the
/// seed / expression / cadence registers are thread-local, so the renders cannot see each other;
/// the event dump is process-global and is never armed here). Without the opt-in variable only
/// the portable cases are rendered, so the always-on path runs no libm-bound case at all.
fn renders() -> &'static [(&'static str, RenderOutput)] {
    static R: OnceLock<Vec<(&'static str, RenderOutput)>> = OnceLock::new();
    R.get_or_init(|| {
        let all = wav_golden_enabled();
        let handles: Vec<_> = GOLDENS
            .iter()
            .filter(|g| all || portable(g.case))
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

/// The render of `case`; panics if this process did not render it (see [`renders`]).
fn rendered(case: &str) -> &'static RenderOutput {
    renders()
        .iter()
        .find(|(c, _)| *c == case)
        .map(|(_, out)| out)
        .unwrap_or_else(|| panic!("golden case {case} was not rendered in this process"))
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

/// Results of the libm functions the render path imports (`nm -D` of this test binary, S122),
/// on fixed inputs, hashed per function: `(name, first 16 hex of SHA-256 of the result bits)`.
/// `black_box` keeps the compiler from folding a call away, so every probe reaches libm.
fn libm_fingerprint() -> Vec<(&'static str, String)> {
    const N: u32 = 512;
    fn probe(name: &'static str, f: impl Fn(u32) -> u64) -> (&'static str, String) {
        let mut bytes = Vec::with_capacity(N as usize * 8);
        for i in 0..N {
            bytes.extend_from_slice(&f(black_box(i)).to_le_bytes());
        }
        (name, sha256_hex(&bytes)[..16].to_string())
    }
    // Inputs: a signed range wide enough to exercise range reduction, a positive range, and a
    // second operand for the two-argument functions.
    let s32 = |i: u32| black_box(i as f32 * 0.113 - 28.9);
    let p32 = |i: u32| black_box(i as f32 * 0.0391 + 0.0007);
    let s64 = |i: u32| black_box(i as f64 * 0.113 - 28.9);
    let p64 = |i: u32| black_box(i as f64 * 0.0391 + 0.0007);
    vec![
        probe("sinf", |i| s32(i).sin().to_bits() as u64),
        probe("cosf", |i| s32(i).cos().to_bits() as u64),
        probe("powf", |i| p32(i).powf(s32(i) * 0.1).to_bits() as u64),
        probe("log10f", |i| p32(i).log10().to_bits() as u64),
        probe("expf", |i| (s32(i) * 0.3).exp().to_bits() as u64),
        probe("exp2f", |i| (s32(i) * 0.3).exp2().to_bits() as u64),
        probe("tanhf", |i| (s32(i) * 0.2).tanh().to_bits() as u64),
        probe("atan2f", |i| s32(i).atan2(s32(N - 1 - i)).to_bits() as u64),
        probe("hypotf", |i| s32(i).hypot(p32(i)).to_bits() as u64),
        probe("sin", |i| s64(i).sin().to_bits()),
        probe("cos", |i| s64(i).cos().to_bits()),
        probe("atan2", |i| s64(i).atan2(s64(N - 1 - i)).to_bits()),
        probe("exp", |i| (s64(i) * 0.3).exp().to_bits()),
        probe("log", |i| p64(i).ln().to_bits()),
        probe("log10", |i| p64(i).log10().to_bits()),
    ]
}

/// The paste-ready replacement line for one case.
fn golden_line(case: &str, out: &RenderOutput, structure: &str, wav: &str) -> String {
    format!(
        "    Golden {{ case: \"{case}\", frames: {}, events: {}, structure_sha256: \"{structure}\", wav_sha256: \"{wav}\" }},",
        out.interleaved.len(),
        out.events.len()
    )
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

/// ALWAYS ON (CI, every OS). Composition + render length: the pre-Phase-0 piece's notes,
/// onsets, velocities, programs and sample count, for every explicit-seed case. Libm-independent
/// (see the module doc); it does not read the opt-in variable except to print WAV hashes.
#[test]
fn golden_structure_portable() {
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for g in GOLDENS.iter().filter(|g| portable(g.case)) {
        let (case, out) = (g.case, rendered(g.case));
        let structure = sha256_hex(&structure_bytes(out));
        let wav = if wav_golden_enabled() {
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
        if wav_golden_enabled() {
            ""
        } else {
            "(wav_sha256 above is the OLD value: regenerate it with the local opt-in WAV golden.)\n"
        }
    );
}

/// LOCAL OPT-IN (never in CI). The exact WAV bytes for every case, plus the structure of the
/// image-derived-seed case. Both are libm-bound. Enable with `AUDIOHAX_WAV_GOLDEN=1` AND
/// `--include-ignored` (see the module doc for the command and what it guards).
#[test]
#[ignore = "LOCAL opt-in guard (WAV bytes are libm-bound). Run: AUDIOHAX_WAV_GOLDEN=1 cargo test --test render_golden_s122 -- --include-ignored"]
fn golden_wav_bytes_local_opt_in() {
    if !wav_golden_enabled() {
        eprintln!(
            "golden_wav_bytes_local_opt_in: SKIPPED, {WAV_GOLDEN_ENV} is not set to 1. This is a \
             LOCAL guard (WAV bytes are libm-bound); the structure golden still ran."
        );
        return;
    }
    let sf2 = std::fs::read(manifest("assets/soundfonts/default.sf2"))
        .expect("assets/soundfonts/default.sf2 (embedded at build time) is readable");
    let sf2_sha = sha256_hex(&sf2);
    assert_eq!(
        sf2_sha, SF2_SHA256,
        "golden_wav_bytes_local_opt_in: CANNOT COMPARE. The embedded SoundFont is not the one the \
         goldens were rendered with, so WAV bytes cannot match. Rebuild with that font."
    );
    let fingerprint = libm_fingerprint();
    let libm_moved: Vec<&str> = fingerprint
        .iter()
        .filter(|(name, hash)| {
            LIBM_FINGERPRINT
                .iter()
                .find(|(n, _)| n == name)
                .is_none_or(|(_, h)| h != hash)
        })
        .map(|(name, _)| *name)
        .collect();
    let mut failures = Vec::new();
    let mut lines = Vec::new();
    for g in GOLDENS {
        let (case, out) = (g.case, rendered(g.case));
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
    let fingerprint_lines: Vec<String> = fingerprint
        .iter()
        .map(|(n, h)| format!("    (\"{n}\", \"{h}\"),"))
        .collect();
    if failures.is_empty() {
        if !libm_moved.is_empty() {
            eprintln!(
                "golden_wav_bytes_local_opt_in: bytes MATCH although the libm fingerprint differs \
                 in {libm_moved:?}; the probes disagree but the render did not. This machine's \
                 fingerprint:\n{}",
                fingerprint_lines.join("\n")
            );
        }
        return;
    }
    if libm_moved.is_empty() {
        panic!(
            "\nRENDERED AUDIO CHANGED (local WAV golden, tests/render_golden_s122.rs):\n  {}\n\n\
             The libm fingerprint matches the recording machine (probe inputs only).\n\
             If, and only if, this change is INTENDED, replace the matching line(s) in GOLDENS with:\n{}\n\
             See the module doc's update procedure.\n",
            failures.join("\n  "),
            lines.join("\n")
        );
    }
    panic!(
        "\nLOCAL WAV GOLDEN RED ON A DIFFERENT LIBM (tests/render_golden_s122.rs):\n  {}\n\n\
         This machine's libm differs from the recording machine's in: {libm_moved:?}.\n\
         A WAV red here may be libm, not sound. Do NOT paste these values into GOLDENS. Trust the\n\
         structure golden, compare on the recording machine, or re-record the WHOLE table and\n\
         LIBM_FINGERPRINT together on one machine. This machine's fingerprint:\n{}\n",
        failures.join("\n  "),
        fingerprint_lines.join("\n")
    );
}

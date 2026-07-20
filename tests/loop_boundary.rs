//! tests/loop_boundary.rs — S103 SEAMLESS-LOOP BOUNDARY GATE.
//!
//! PROPERTY VALIDATED: a `render --wav --loop` cue, played on repeat, introduces **no silent
//! gap at the loop seam longer than the worst gap already inside the music**. That is the
//! defect the taste gate (§10) refused S102's renders on: every cue stopped, ~1.0–1.4 s of
//! dead air, then a hard-onset restart.
//!
//! WHY BINARY-LEVEL. The seeded composition → absolute-ms timeline step
//! (`build_step_event_timeline`) is bin-private in `src/main.rs` and is NOT exported by the
//! library, and the loop period is derived from the composed plan inside `run_render_wav`. A
//! library-level test could only re-implement bin-private code — the dead-path defect §6
//! caught in S102, where a test exercises a function nothing ships. Driving
//! `CARGO_BIN_EXE_audiohax` exercises the REAL shipped path, including the real shipped
//! `assets/mappings.json` that `run_render_wav` loads.
//!
//! ── HOW THE MEASUREMENT WORKS, AND WHAT IT CANNOT SAY ─────────────────────────────────────
//!
//! Short-time RMS envelope (20 ms hop) → longest contiguous run below [`SILENCE_DBFS`].
//! Measured over a THREE-LAP concatenation of the shipped loop (so the seam is inside the
//! signal, twice) and compared against the same statistic over the cue's own interior.
//!
//! Self-normalizing by construction: each cue is judged against its OWN rest structure, so a
//! sparse cue cannot false-pass on a lax absolute threshold and a busy one cannot false-fail.
//!
//! **A structural caveat that must not be misread as corroboration.** The 3-lap concatenation
//! CONTAINS the body, so `longest_gap(3lap) >= longest_gap(body)` holds *identically*, for
//! every possible input. The assertion `longest_gap(3lap) <= longest_gap(body)` is therefore
//! an assertion of EXACT EQUALITY, and a passing run can only ever report equality. Observing
//! "the folded output passes at exact equality with the body" is thus the *definition* of
//! passing, not independent evidence that the fix is good. The test is still a real
//! constraint — it says the seam adds nothing the music does not already do — but its
//! equality result carries no information beyond the pass/fail bit.
//!
//! Because a gate that can only pass is worthless, [`the_gate_discriminates_pre_fix`] pins
//! the other half: the SAME instrument, on the SAME images, must FAIL on the pre-fix
//! (non-`--loop`) path. That test is what makes a green here mean something.
//!
//! WHAT THIS GATE DOES NOT COVER, BY DESIGN:
//!   * **A wrap-step / click test would NOT catch this defect** and is not a substitute:
//!     today's broken seam is silence → soft attack, so the sample-domain derivative at the
//!     seam is SMALL and a click test passes on broken output.
//!   * **Ring-out truncation** is invisible to any seam-only measurement — a cue cut mid-decay
//!     has no gap at all. That property is enforced in `run_render_wav` itself (the
//!     decay-completion guard), at every real render, not here.
//!   * **The delivery chain.** These assertions run on the binary's own 44.1 kHz/16-bit
//!     output; the deliverables are additionally loudnorm'd and resampled to 48 kHz/24-bit.
//!
//!     ⚠ S103 §5 (SECOND KEY) — §2's R3 IS HALF RIGHT, ON A FALSE PREMISE. R3 asserted the
//!     chain runs loudnorm in `dynamic` mode. It does not: `s102-smashbob-wavs/RENDER-LOG.md`
//!     line 134 records "ffmpeg two-pass `loudnorm`, **linear**". MEASURED, by running both
//!     chains end-to-end on a folded `example.jpg --loop` cue and re-taking the seam statistic
//!     on the 48/24 output:
//!       - two-pass LINEAR (as shipped): seam-to-body margin 10.6 dB — IDENTICAL to the
//!         binary's own 10.6 dB. Linear mode applies one constant gain, so it CANNOT introduce
//!         a time-varying seam dip. R3 does not apply to the artifact that shipped.
//!       - single-pass DYNAMIC (R3's assumption): margin 8.9 dB — degraded by 1.7 dB. The
//!         MECHANISM R3 describes is real; it was simply never the mode in use.
//!     Note the gap statistic stayed 0 ms through all three stages — only the continuous
//!     margin in [`loop_seam_level_is_no_deeper_than_the_body_s_own_rests`] saw the dynamic
//!     degradation at all.
//!
//!     STILL UNDISCHARGED: nothing in this suite runs the chain. Closing it needs ffmpeg in
//!     the harness (present on this box) plus a 24-bit decode path, and it must re-assert that
//!     the chain stayed `linear=true` — that flag is the whole reason R3 is inert.
//!
//! Default feature set only (the binary needs the pure-Rust `synth` + `pure-analysis`
//! defaults), matching `tests/render_cli_determinism_s55.rs`.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Windows below this are treated as silence. −50 dBFS sits far under any sounding note in
/// these renders (whose true peak is ≈ −13 dBFS) and comfortably over the dither/noise floor.
const SILENCE_DBFS: f32 = -50.0;

/// Short-time RMS hop, in milliseconds. 20 ms resolves the ~1.0–1.4 s failure with ~70x
/// margin while staying long enough that a single waveform zero-crossing cannot read as
/// silence.
const HOP_MS: usize = 20;

/// Region excluded from the "body" statistic at each end of a single lap, so that the body
/// statistic describes the music's INTERIOR rest structure only.
///
/// ⚠ S103 §5 (SECOND KEY) CORRECTION: 2000 -> 4500 ms. The constant shipped at 2000 ms
/// justified by a fold region of "≈ 1.6 s". That figure belongs to the OLD 1500 ms tail: 1500 ms
/// plus cross-step overlap. §2's fix raises the tail to 4000 ms **on the loop path**, and the
/// binary itself reports what it actually folded — MEASURED 4058 / 4000 / 4028 / 4040 ms for
/// Img1 / Img2 / Img3 / example. The fold region is therefore ~4.0 s, not 1.6 s, and a 2000 ms
/// exclusion left up to 2058 ms of FOLDED region inside the "body" window. That is the
/// instrument measuring the seam's own influence and then using it as the reference the seam is
/// judged against — a self-referential comparison, and precisely the alignment class of error
/// §2's R5 was about.
///
/// VERIFIED INERT TODAY, FIXED ANYWAY: sweeping the exclusion across 2000/3000/4500/5000 ms
/// moves neither the body gap (Img1 160 ms, the other six 0 ms at every width) nor the
/// seam-level margins (identical to 0.1 dB). So no verdict changes — but the stated invariant
/// was false, and a future cue with rests in that band would have been judged against a
/// contaminated reference. 4500 ms clears the measured worst case (4058 ms) with margin while
/// leaving 7.6 s of interior on the SHORTEST cue (example, 16.59 s), well over the
/// `mid_end > mid_start + 5` liveness guard in `measure`.
const BODY_EDGE_EXCLUSION_MS: usize = 4500;

/// The sweep. Four images x both reverb settings = 8 configurations.
///
/// Sweeping is REQUIRED, not thorough-for-its-own-sake: S102 established that the two
/// ensembles fail in OPPOSITE directions (short-decay patches leave dead air; long-decay
/// patches truncate), so a fixture picking either one alone would certify a half-fix. Reverb
/// on/off is the axis that most changes tail length, which is precisely what the fold moves.
const IMAGES: [&str; 4] = [
    "assets/images/AudioHaxImg1.jpg",
    "assets/images/AudioHaxImg2.jpg",
    "assets/images/AudioHaxImg3.jpg",
    "assets/images/example.jpg",
];
const REVERBS: [&str; 2] = ["off", "on"];

/// Shrinks the pure-analysis scan work only. On the COMPOSED render path the audible timeline
/// length comes from the plan, not `--steps` (`main.rs`), so this does NOT change the
/// composition or weaken the property under test — same rationale as
/// `render_cli_determinism_s55.rs`.
const STEPS: &str = "6";

/// A decoded mono signal plus its sample rate.
struct Signal {
    samples: Vec<f32>,
    sample_rate: u32,
}

/// Render `image` through the REAL shipped binary and decode the result to mono.
///
/// `loop_cue` selects the path under test. Returns `Err` with the binary's stderr when the
/// render itself fails, so a guard firing in `run_render_wav` surfaces as a readable message
/// rather than a decode panic.
fn render(image: &str, reverb: &str, loop_cue: bool, tag: &str) -> Result<Signal, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "audiohax-loop-{tag}-{}-{nanos}.wav",
        std::process::id()
    ));

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_audiohax"));
    cmd.arg("render")
        .arg(image)
        .arg("--wav")
        .arg(&path)
        .arg("--seed")
        .arg("102")
        .arg("--steps")
        .arg(STEPS)
        .arg("--reverb")
        .arg(reverb);
    if loop_cue {
        cmd.arg("--loop");
    }
    let out = cmd.output().expect("spawn audiohax");
    if !out.status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }

    let sig = decode_mono(&path);
    let _ = std::fs::remove_file(&path);
    Ok(sig)
}

/// Decode a 16-bit stereo WAV to mono f32, asserting the header rather than assuming it.
fn decode_mono(path: &std::path::Path) -> Signal {
    let mut reader = hound::WavReader::open(path).expect("open rendered WAV");
    let spec = reader.spec();
    // Read from the header; never assumed. The binary hardcodes 44.1 kHz/16-bit — the
    // 48 kHz/24-bit deliverables are produced by a LATER ffmpeg stage, outside this binary.
    assert_eq!(spec.sample_rate, 44_100, "binary renders at 44.1 kHz");
    assert_eq!(spec.bits_per_sample, 16, "binary renders 16-bit");
    assert_eq!(spec.channels, 2, "binary renders stereo");

    let raw: Vec<i16> = reader
        .samples::<i16>()
        .map(|s| s.expect("sample"))
        .collect();
    let samples = raw
        .chunks_exact(2)
        .map(|lr| (lr[0] as f32 + lr[1] as f32) / 2.0 / i16::MAX as f32)
        .collect();
    Signal {
        samples,
        sample_rate: spec.sample_rate,
    }
}

/// Short-time RMS envelope in dBFS, one entry per [`HOP_MS`] window.
fn envelope_dbfs(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let hop = HOP_MS * sample_rate as usize / 1000;
    samples
        .chunks(hop)
        .map(|w| {
            let rms = (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt();
            20.0 * rms.max(1e-12).log10()
        })
        .collect()
}

/// Longest contiguous run of sub-[`SILENCE_DBFS`] windows, in milliseconds.
fn longest_gap_ms(env: &[f32]) -> usize {
    let (mut best, mut run) = (0usize, 0usize);
    for &d in env {
        if d < SILENCE_DBFS {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best * HOP_MS
}

/// The two statistics the gate compares, plus the liveness facts that prove they are not
/// being read off a vacuum.
struct LoopStats {
    three_lap_gap_ms: usize,
    body_gap_ms: usize,
}

/// Measure one rendered cue: build the 3-lap concatenation, take both gap statistics, and
/// assert the liveness preconditions that stop a green from being a green-on-silence.
fn measure(sig: &Signal, label: &str) -> LoopStats {
    // ── LIVENESS: a silent or near-silent buffer would trivially satisfy some orderings of
    //    the gap comparison. Refuse to report a verdict on one. ──
    let peak = sig.samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    let peak_dbfs = 20.0 * peak.max(1e-12).log10();
    assert!(
        peak_dbfs > -40.0,
        "{label}: peak {peak_dbfs:.1} dBFS — buffer is effectively silent, no verdict is \
         meaningful"
    );

    let three_lap: Vec<f32> = sig
        .samples
        .iter()
        .chain(sig.samples.iter())
        .chain(sig.samples.iter())
        .copied()
        .collect();
    let three_env = envelope_dbfs(&three_lap, sig.sample_rate);
    let sounding = three_env.iter().filter(|&&d| d > -30.0).count();
    assert!(
        sounding >= 3,
        "{label}: only {sounding} windows above −30 dBFS — nothing is playing, no verdict is \
         meaningful"
    );

    // ── BOTH statistics are read off the SAME envelope, at the SAME window alignment. ──
    //
    // This is NOT incidental. Measuring the body on a single lap and the seam on the 3-lap
    // concatenation — the obvious construction — compares two envelopes computed at
    // DIFFERENT window phases, because the lap length is not an integer multiple of the hop
    // (e.g. 1_234_359 frames / 882 = 1399.5 windows). A true gap of ~150 ms then quantizes to
    // 140 ms at one phase and 160 ms at the other, and the test reports a 20 ms "seam gap"
    // that does not exist in the signal. That artifact was observed: AudioHaxImg1/reverb=on
    // reported 3-lap 160 ms vs body 140 ms while direct inspection found NO gap at the seam
    // at all — the longest run anywhere was 140 ms, at 11.7 s, 15.4 s and 19.2 s, all
    // interior.
    //
    // Taking the body from the MIDDLE lap of the same concatenation removes the artifact and
    // makes the containment relation exact rather than approximate.
    let hop = HOP_MS * sig.sample_rate as usize / 1000;
    let lap = sig.samples.len();
    let edge = BODY_EDGE_EXCLUSION_MS / HOP_MS;
    let mid_start = lap / hop + edge;
    let mid_end = (2 * lap) / hop - edge;
    assert!(
        mid_end > mid_start + 5 && mid_end <= three_env.len(),
        "{label}: cue too short to have an interior after edge exclusion (lap {} windows)",
        lap / hop
    );
    let body = &three_env[mid_start..mid_end];

    LoopStats {
        three_lap_gap_ms: longest_gap_ms(&three_env),
        body_gap_ms: longest_gap_ms(body),
    }
}

/// ── THE GATE ─────────────────────────────────────────────────────────────────────────────
///
/// Across all 8 configurations, a `--loop` cue played three times must introduce no silence
/// run longer than the worst silence run already inside that same cue.
#[test]
fn loop_seam_adds_no_gap_beyond_the_cue_s_own_rest_structure() {
    let mut failures = Vec::new();
    for image in IMAGES {
        for reverb in REVERBS {
            let label = format!("{image} reverb={reverb}");
            let sig = match render(image, reverb, true, "green") {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{label}: render failed: {e}"));
                    continue;
                }
            };
            let st = measure(&sig, &label);
            println!(
                "  [--loop  ] {label}: 3-lap gap {} ms, body gap {} ms",
                st.three_lap_gap_ms, st.body_gap_ms
            );
            if st.three_lap_gap_ms > st.body_gap_ms {
                failures.push(format!(
                    "{label}: seam gap {} ms exceeds body gap {} ms",
                    st.three_lap_gap_ms, st.body_gap_ms
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "--loop cues introduced a seam gap:\n  {}",
        failures.join("\n  ")
    );
}

/// ── THE DISCRIMINATION WITNESS ───────────────────────────────────────────────────────────
///
/// The gate above is only meaningful if the instrument can FAIL. This runs the identical
/// measurement over the pre-fix (non-`--loop`) path and requires it to fail on EVERY
/// configuration — which is the S102 defect, reproduced under test.
///
/// Without this, the gate would be indistinguishable from one that always passes, and a later
/// refactor that quietly neutered the measurement would go unnoticed.
#[test]
fn the_gate_discriminates_pre_fix() {
    let mut passed_when_it_should_not = Vec::new();
    for image in IMAGES {
        for reverb in REVERBS {
            let label = format!("{image} reverb={reverb}");
            let sig = render(image, reverb, false, "red").expect("pre-fix render");
            let st = measure(&sig, &label);
            println!(
                "  [no --loop] {label}: 3-lap gap {} ms, body gap {} ms",
                st.three_lap_gap_ms, st.body_gap_ms
            );
            if st.three_lap_gap_ms <= st.body_gap_ms {
                passed_when_it_should_not.push(format!(
                    "{label}: 3-lap gap {} ms <= body gap {} ms",
                    st.three_lap_gap_ms, st.body_gap_ms
                ));
            }
        }
    }
    assert!(
        passed_when_it_should_not.is_empty(),
        "the pre-fix path PASSED the loop gate — the measurement is not discriminating, so a \
         green from the gate proves nothing:\n  {}",
        passed_when_it_should_not.join("\n  ")
    );
}

/// The loop path must be shorter than the default path by exactly the ring-out it folded
/// away — i.e. the boundary genuinely MOVED to the musical period rather than the tail simply
/// being trimmed to silence.
///
/// ⚠ S103 §5: this test, and the gap gate above, are BOTH satisfied by plain truncation. See
/// [`loop_fold_is_not_mere_truncation`] for the assertion that separates them.
#[test]
fn loop_render_is_trimmed_to_the_musical_period() {
    let plain = render(IMAGES[0], "off", false, "len-plain").expect("plain render");
    let looped = render(IMAGES[0], "off", true, "len-loop").expect("loop render");
    assert!(
        looped.samples.len() < plain.samples.len(),
        "loop render ({}) must be shorter than the default render ({})",
        looped.samples.len(),
        plain.samples.len()
    );
    // The default boundary is `last_NoteOff + 1500 ms`, and the period is the grid width, so
    // the difference is the cross-step overlap plus that 1.5 s — on the order of 1.5–1.7 s.
    let delta_ms = (plain.samples.len() - looped.samples.len()) * 1000 / plain.sample_rate as usize;
    assert!(
        (1_400..=1_800).contains(&delta_ms),
        "expected the trim to be the 1.5 s tail plus cross-step overlap, got {delta_ms} ms"
    );
}

/// `--loop` at non-unity gain must be REFUSED, not silently approximated: `apply_gain_softclip`
/// is `tanh` off unity, and the fold's exactness argument requires linearity.
#[test]
fn loop_refuses_non_unity_gain() {
    let err = render(IMAGES[0], "off", true, "gain")
        .map(|_| String::new())
        .unwrap_or_else(|e| e);
    // The helper does not pass --gain, so the above must SUCCEED; drive the refusal directly.
    assert!(
        err.is_empty(),
        "unity-gain loop render should succeed: {err}"
    );

    let path = std::env::temp_dir().join(format!("audiohax-loop-gain-{}.wav", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_audiohax"))
        .args(["render", IMAGES[0], "--wav"])
        .arg(&path)
        .args(["--seed", "102", "--steps", STEPS, "--loop", "--gain", "1.5"])
        .output()
        .expect("spawn");
    let _ = std::fs::remove_file(&path);
    assert!(!out.status.success(), "--loop --gain 1.5 must be refused");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unity") && stderr.contains("gain"),
        "refusal must explain the linearity requirement, got: {stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════
// S103 §5 (SECOND KEY) — ADDED DURING THE SEAL. See the report for the measurements.
// ═══════════════════════════════════════════════════════════════════════════════════════════

/// ── THE FOLD-VS-TRUNCATION WITNESS ───────────────────────────────────────────────────────
///
/// WHY THIS EXISTS. §2's own R6 note says a pass of the gap gate can only ever report exact
/// equality, and pinned the other half with `the_gate_discriminates_pre_fix`. That witness is
/// real but COARSE: it separates "the boundary moved" from "the boundary did not move". It
/// does NOT separate the wrap-fold from simply TRUNCATING the buffer at the period.
///
/// MEASURED, S103 §5: replacing the whole wrap-accumulate with `interleaved[..period*2]
/// .to_vec()` leaves every one of the 8 configurations' gap statistics BIT-FOR-BIT unchanged
/// (`AudioHaxImg1` 160/160, the other six 0/0), so both `loop_seam_adds_no_gap_...` and
/// `loop_render_is_trimmed_to_the_musical_period` still pass. The entire novel operation — the
/// wrap-accumulate that is the actual fix — was uncovered by the suite that shipped with it.
/// Truncation is strictly WORSE than the fold (it discards the ring-out instead of relocating
/// it, which is exactly the failure `run_render_wav`'s decay guard exists to prevent), so a
/// gate blind to the difference is the S102 vacuum in a new place.
///
/// HOW IT DISCRIMINATES. The two renders share a deterministic prefix: `render_events_to_stereo`
/// renders the same events through the same synth and `tail_ms` only decides when it STOPS, so
/// the default render and the loop render's pre-fold buffer are identical sample-for-sample as
/// far as the shorter one goes. Therefore:
///
///   * FOLD      ⇒ `loop[i] = plain[i] + plain[i+P] + …` over the fold region — it MUST differ.
///   * TRUNCATE  ⇒ `loop[i] = plain[i]` everywhere — it CANNOT differ.
///
/// and past the fold region both must be identical again. Asserting BOTH halves is what makes
/// this a real comparison rather than a "they differ because everything differs" tautology: the
/// interior-identity half proves the two signals are the same underlying render, so the
/// head-difference half can only be the fold.
#[test]
fn loop_fold_is_not_mere_truncation() {
    for image in IMAGES {
        let plain = render(image, "off", false, "fold-plain").expect("plain render");
        let looped = render(image, "off", true, "fold-loop").expect("loop render");
        let sr = plain.sample_rate as usize;

        // ── HALF 1: the head MUST carry folded-in ring-out. ──
        // The binary reports folding ~4.0–4.06 s of ring-out; measure the first 1000 ms, well
        // inside that region on every configuration.
        let head = 1000 * sr / 1000;
        assert!(
            looped.samples.len() > head && plain.samples.len() > head,
            "[{image}] renders too short to compare"
        );
        let resid: f32 = (0..head)
            .map(|i| (looped.samples[i] - plain.samples[i]).powi(2))
            .sum::<f32>()
            / head as f32;
        let resid_db = 20.0 * resid.sqrt().max(1e-12).log10();
        let plain_rms: f32 =
            (plain.samples[..head].iter().map(|s| s * s).sum::<f32>() / head as f32).sqrt();
        let plain_db = 20.0 * plain_rms.max(1e-12).log10();
        assert!(
            resid_db > -80.0,
            "[{image}] THE FOLD DID NOT HAPPEN: the first {} ms of the --loop render is \
             identical to the default render (residual {resid_db:.1} dBFS vs signal \
             {plain_db:.1} dBFS). That is what plain truncation to the period produces. The \
             wrap-accumulate must ADD the ring-out onto the head.",
            head * 1000 / sr
        );
        println!(
            "  [FOLD] {image}: head residual {resid_db:.1} dBFS against signal {plain_db:.1} \
             dBFS — ring-out is present"
        );

        // ── HALF 2: past the fold region both renders must be the SAME SIGNAL again. ──
        // This is the control. It fails if the loop path perturbed the whole render (a global
        // gain change, a different seed, a resample), which would make Half 1 pass for the
        // wrong reason.
        let probe_start = 6_000 * sr / 1000; // 6 s — beyond the ~4.06 s max fold region
        let probe_len = 500 * sr / 1000;
        if looped.samples.len() > probe_start + probe_len {
            let diff = (probe_start..probe_start + probe_len)
                .filter(|&i| looped.samples[i] != plain.samples[i])
                .count();
            assert_eq!(
                diff, 0,
                "[{image}] the --loop render differs from the default render OUTSIDE the fold \
                 region ({diff}/{probe_len} samples at 6.0–6.5 s). The loop path must relocate \
                 ring-out onto the head and change NOTHING else; a difference here means the \
                 head-residual assertion above is not measuring the fold."
            );
        }
    }
}

/// ── SEAM LEVEL, MEASURED CONTINUOUSLY RATHER THAN THRESHOLDED ────────────────────────────
///
/// WHY THIS EXISTS. The gap gate reduces each cue to "longest run below −50 dBFS". MEASURED,
/// S103 §5: on 6 of the 8 configurations BOTH of its statistics are 0 ms, so the comparison
/// `three_lap <= body` degenerates to `0 <= 0` and the surviving criterion is the ABSOLUTE one
/// — "never dips below −50 dBFS" — not the self-normalising one it is documented as. Its
/// sensitivity floor was measured by injecting a 600 ms level dip at the seam of
/// `example.jpg`: the gate first reacts at about −47 dBFS RMS, so a dip to −44 dBFS — 13 dB
/// under that cue's median sounding level, 0.63% amplitude — passes it with 0 ms vs 0 ms.
///
/// This test replaces the threshold with a CONTINUOUS comparison: the quietest window anywhere
/// near the seam, against the quietest window anywhere in the cue's own interior. It is
/// genuinely self-normalising (a cue with deep rests is allowed a deep seam) and it has real
/// margin instead of a pass/fail bit.
///
/// MEASURED MARGINS on the shipped fold, all 8 configurations — seam minus body-minimum:
/// `Img1` −193.6/−38.5 dB, `Img2` −2.0/−2.4 dB, `Img3` −6.1/−6.1 dB, `example` −10.6/−9.4 dB
/// (off/on). Every seam is SHALLOWER than that cue's own deepest interior rest; the tightest
/// margin is 2.0 dB on `AudioHaxImg2`. The 1.0 dB tolerance below sits inside that.
///
/// WHAT IT STILL CANNOT SAY: a seam dip that stays within the cue's own worst rest is passed by
/// construction, and that is the intended self-normalising behaviour, not a hole. Whether the
/// fold's superimposed ring-out READS musically is §10's, not this gate's.
#[test]
fn loop_seam_level_is_no_deeper_than_the_body_s_own_rests() {
    /// Tolerance on the seam-vs-body minimum comparison. The tightest measured margin is
    /// 2.0 dB (`AudioHaxImg2`), so this admits float/platform variance without admitting a
    /// real dip.
    const SEAM_TOLERANCE_DB: f32 = 1.0;
    /// Half-width of the seam neighbourhood examined at each lap boundary.
    const SEAM_WINDOW_MS: usize = 500;

    let mut failures = Vec::new();
    for image in IMAGES {
        for reverb in REVERBS {
            let label = format!("{image} reverb={reverb}");
            let sig = render(image, reverb, true, "seamlvl").expect("loop render");
            let three: Vec<f32> = sig
                .samples
                .iter()
                .chain(sig.samples.iter())
                .chain(sig.samples.iter())
                .copied()
                .collect();
            let env = envelope_dbfs(&three, sig.sample_rate);
            let hop = HOP_MS * sig.sample_rate as usize / 1000;
            let lap = sig.samples.len() / hop;
            let w = SEAM_WINDOW_MS / HOP_MS;
            let edge = BODY_EDGE_EXCLUSION_MS / HOP_MS;

            let min_of = |s: &[f32]| s.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let body_min = min_of(&env[lap + edge..2 * lap - edge]);
            let seam_min =
                min_of(&env[lap - w..lap + w]).min(min_of(&env[2 * lap - w..2 * lap + w]));

            println!(
                "  [SEAM LVL] {label}: seam min {seam_min:.1} dBFS vs body min {body_min:.1} \
                 dBFS (margin {:.1} dB)",
                seam_min - body_min
            );
            if seam_min < body_min - SEAM_TOLERANCE_DB {
                failures.push(format!(
                    "{label}: seam dips to {seam_min:.1} dBFS, {:.1} dB below the deepest rest \
                     anywhere in the cue's own interior ({body_min:.1} dBFS)",
                    body_min - seam_min
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "--loop cues dip at the seam below their own interior rest structure:\n  {}",
        failures.join("\n  ")
    );
}

/// ── THE PERIOD IS THE MUSICAL GRID, TO THE FRAME ─────────────────────────────────────────
///
/// §2's first self-named distrust point: the loop period is computed with ROUNDING
/// (`(period_ms * sr + 500) / 1000`) while `render_events_to_stereo` sizes its buffer by
/// TRUNCATION (`total_ms * sr / 1000`), and §2 did not prove the two agree to the frame.
/// Nothing asserted the period at all — it was only ever printed.
///
/// RE-DERIVED, S103 §5. The two conventions differ by at most ONE frame (22.7 µs), and only
/// when `period_ms % 10 >= 5`: `period_ms * 44100 mod 1000 == 100 * (period_ms mod 10)`, so the
/// `+500` changes the quotient only for the top half of that residue. MEASURED on all four
/// shipped cues — 27990, 23112, 16884, 16590 ms — every residue is 0, 2, 4, 0, so rounding and
/// truncation give BIT-IDENTICAL results here and the worry is discharged.
///
/// ⚠ THE REAL QUANTISATION IS 441x LARGER AND §2 DID NOT NAME IT. `render_events_to_stereo`
/// applies events on `BLOCK = 441`-frame boundaries — exactly 10 ms at 44.1 kHz — so step `N`
/// lands at frame `441 * floor(N * grid_ms / 10)`, not at its exact grid position. The seam's
/// inter-onset interval is therefore MEASURED 940 / 642 / 474 / 400 ms against nominal
/// 933 / 642 / 469 / 395. That error is bounded by the 10 ms block, is non-cumulative, and is
/// the SAME quantisation the interior already carries (Img1's interior steps alternate
/// 930/940 ms about a 933 ms nominal). So the seam is no less accurate than the music around
/// it — but the fold is exact only to ±10 ms, not to the frame, and that is a property of the
/// frozen engine's block loop, not of the fix.
#[test]
fn loop_period_equals_the_musical_grid() {
    for image in IMAGES {
        let path = std::env::temp_dir().join(format!(
            "audiohax-loop-period-{}-{}.wav",
            std::process::id(),
            image.replace('/', "_")
        ));
        let out = Command::new(env!("CARGO_BIN_EXE_audiohax"))
            .args(["render", image, "--wav"])
            .arg(&path)
            .args([
                "--seed", "102", "--steps", STEPS, "--reverb", "off", "--loop",
            ])
            .output()
            .expect("spawn");
        assert!(out.status.success(), "loop render failed for {image}");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

        // "--loop → period {steps} steps x {grid} ms = {period_ms} ms ({frames} frames)"
        let line = stdout
            .lines()
            .find(|l| l.contains("--loop → period"))
            .unwrap_or_else(|| panic!("[{image}] binary printed no period line:\n{stdout}"));
        let nums: Vec<u128> = line
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        assert!(nums.len() >= 4, "[{image}] unparseable period line: {line}");
        let (steps, grid_ms, period_ms, period_frames) = (nums[0], nums[1], nums[2], nums[3]);

        // The period IS the grid width — not the last NoteOff, not a tuned constant.
        assert_eq!(
            period_ms,
            steps * grid_ms,
            "[{image}] period {period_ms} ms != total_steps {steps} x grid {grid_ms} ms"
        );
        // Rounding and truncation must agree here; if a future tempo makes them disagree this
        // fires and the ±1-frame choice has to be made deliberately.
        let rounded = (period_ms * 44_100 + 500) / 1000;
        let truncated = (period_ms * 44_100) / 1000;
        assert_eq!(
            rounded, truncated,
            "[{image}] period {period_ms} ms puts the rounding and truncating frame conventions \
             1 frame apart ({rounded} vs {truncated}). They agree on every cue shipped to date; \
             a tempo that separates them needs a deliberate choice, not a silent one."
        );
        assert_eq!(
            period_frames as u128, rounded,
            "[{image}] reported period_frames {period_frames} != derived {rounded}"
        );

        // And the WAV that actually shipped must be EXACTLY that long.
        let sig = decode_mono(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            sig.samples.len() as u128,
            period_frames,
            "[{image}] the written WAV is {} frames but the loop period is {period_frames}. The \
             file that loops must BE the period.",
            sig.samples.len()
        );
        println!(
            "  [PERIOD] {image}: {steps} steps x {grid_ms} ms = {period_ms} ms = \
             {period_frames} frames, WAV length matches exactly"
        );
    }
}

//! S125 ITEM 3 (operator D-51, D-56): "Shrink picture" / `--shrink`, built to the UI Designer's
//! final spec (`s125-UI4-shrink-final-spec.md`), §13's filter and rounding (`s125-13`, L-64) and
//! Shielding's bounds (`s125-SH5-prerelease-spec.md` Part C).
//!
//! Library and CLI only (the window's tests live in `src/gui/app.rs`, `src/gui/job.rs` and
//! `tests/gui_shrink_s125.rs`). Pictures are built in the test, never committed. Files go under
//! `CARGO_TARGET_TMPDIR`, never `/tmp`.
//!
//! What is pinned here:
//! * SK1: the default (no flag, `--shrink 100`, `Shrink::Full`) is byte-identical to the pre-S125
//!   path and makes NO resize call (the library's per-thread resize counter).
//! * SK13: the flag is LIVE: 100 % and 50 % give different WAV bytes (the `Hidden` schema row
//!   escapes the inert-knob forcing test, so this one is owed).
//! * SK9: determinism at every level, and the CLI equals the library at every level.
//! * §13 SHOULD 2 (M-2): a golden that pins WHICH piece a shrink plays: the FNV-1a-64 of a fixed
//!   fixture's shrunk RGB buffer at 75 / 50 / 25 %. A change of filter or of the `image` crate
//!   re-draws every shrunk piece; this turns it red.
//! * SK2 / SH5 S5: the CLI's closed value set; SK7 / SH5 S2–S3: the floor refusal, before any work;
//!   SK10: the notice on stderr only, once, stdout's format unchanged; SK11: no panic.
#![cfg(all(feature = "pure-analysis", feature = "synth"))]

use audiohax::cli::{Cli, Command as CliCommand};
use audiohax::mapping_loader::{default_mappings, MappingTable};
use audiohax::pure_analysis::PureImage;
use audiohax::render::{
    render, render_shrunk, ImageInput, RenderError, RenderOutput, RenderRequest,
};
use audiohax::shrink::{
    apply, fits, resizes_on_this_thread, shrunk_size, Shrink, ShrinkError, SHRINK_MIN_SIDE,
};
use audiohax::synth_sink::SoundFontSource;
use clap::Parser;
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_audiohax");

// ── fixtures ────────────────────────────────────────────────────────────────────────────────

/// A deterministic, structured RGB picture (gradients, a checker and diagonal bands), so the
/// analysis has texture, edges and colour to respond to.
fn structured(w: u32, h: u32) -> RgbImage {
    RgbImage::from_fn(w, h, |x, y| {
        let checker = ((x / 6) + (y / 6)) % 2 == 0;
        let band = ((x + 2 * y) / 9) % 3;
        Rgb([
            ((x * 255) / w.max(1)) as u8,
            if checker { 200 } else { 40 },
            (((y * 255) / h.max(1)) as u8).wrapping_add(band as u8 * 60),
        ])
    })
}

fn png(img: &RgbImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)
        .expect("encode png");
    out.into_inner()
}

/// FNV-1a 64 (the same instrument §13 used for run-to-run determinism).
fn fnv64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s125-shrink-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

fn mappings() -> MappingTable {
    default_mappings().expect("built-in mappings")
}

/// The request and shrink clap builds for `audiohax render <args…>`: the real CLI path.
fn cli_request(args: &[&str]) -> (RenderRequest, Shrink) {
    let mut argv = vec!["audiohax", "render"];
    argv.extend_from_slice(args);
    match Cli::try_parse_from(argv).expect("parses").command {
        CliCommand::Render(r) => (RenderRequest::from(&r), r.shrink.shrink),
        other => panic!("not render: {other:?}"),
    }
}

fn lib_render(req: &RenderRequest, shrink: Shrink, bytes: &[u8]) -> RenderOutput {
    render_shrunk(
        req,
        shrink,
        ImageInput::Bytes(bytes),
        &mappings(),
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("render")
}

struct Out {
    rc: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run the CLI in `cwd` (it reads `assets/mappings.json` relative to the working directory, so the
/// crate root is used), stdin closed; FAIL after `secs`.
fn run(args: &[&str], secs: u64) -> Out {
    let mut child = Command::new(BIN)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let t0 = Instant::now();
    while child.try_wait().expect("wait").is_none() {
        if t0.elapsed() > Duration::from_secs(secs) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{args:?}: still running after {secs} s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let o = child.wait_with_output().expect("output");
    Out {
        rc: o.status.code(),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn write_png(dir: &Path, name: &str, img: &RgbImage) -> String {
    let p = dir.join(name);
    std::fs::write(&p, png(img)).expect("write png");
    p.display().to_string()
}

/// Replace every run of ASCII digits with `#` (compare line FORMATS, not values).
fn digits_masked(s: &str) -> String {
    let mut out = String::new();
    let mut in_digits = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                out.push('#');
            }
            in_digits = true;
        } else {
            out.push(c);
            in_digits = false;
        }
    }
    out
}

// ── library ─────────────────────────────────────────────────────────────────────────────────

/// SK1 (library half): `Shrink::Full` is the pre-S125 `render`, byte for byte, and no resize runs.
/// The CLI's `--shrink 100` and an absent flag parse to `Shrink::Full` and the same request.
#[test]
fn sk1_full_is_the_default_byte_for_byte_and_makes_no_resize_call() {
    let bytes = png(&structured(120, 80));
    let (req, absent) = cli_request(&["--seed", "7"]);
    let (req100, hundred) = cli_request(&["--seed", "7", "--shrink", "100"]);
    assert_eq!(absent, Shrink::Full);
    assert_eq!(hundred, Shrink::Full);
    assert_eq!(
        req, req100,
        "--shrink 100 changes nothing else in the request"
    );
    let before = resizes_on_this_thread();
    let plain = render(
        &req,
        ImageInput::Bytes(&bytes),
        &mappings(),
        SoundFontSource::Bundled,
        &mut |_| Ok(()),
    )
    .expect("render");
    let full = lib_render(&req, Shrink::Full, &bytes);
    assert_eq!(
        resizes_on_this_thread(),
        before,
        "the default path called the resizer"
    );
    assert_eq!(plain.interleaved, full.interleaved);
    assert_eq!(plain.events, full.events);
    assert_eq!(plain.summary, full.summary);
    // Control: the counter does move when a shrink runs (it is not a dead instrument).
    let _ = lib_render(&req, Shrink::Percent50, &bytes);
    assert_eq!(resizes_on_this_thread(), before + 1);
}

/// SK13: the flag is live. 100 % and each shrink level give different WAV samples.
#[test]
fn sk13_every_shrink_level_changes_the_piece() {
    let bytes = png(&structured(120, 80));
    let (req, _) = cli_request(&["--seed", "7"]);
    let full = lib_render(&req, Shrink::Full, &bytes);
    for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
        let shrunk = lib_render(&req, s, &bytes);
        assert_ne!(
            full.interleaved, shrunk.interleaved,
            "{s:?} rendered the full-size piece: the shrink is inert"
        );
    }
}

/// SK9 (library half): same picture + same shrink + same Variation ⇒ the same piece.
#[test]
fn sk9_each_level_is_deterministic() {
    let bytes = png(&structured(120, 80));
    let (req, _) = cli_request(&["--seed", "7"]);
    for s in Shrink::ALL {
        let a = lib_render(&req, s, &bytes);
        let b = lib_render(&req, s, &bytes);
        assert_eq!(a.interleaved, b.interleaved, "{s:?}");
        assert_eq!(a.events, b.events, "{s:?}");
    }
}

/// §13 SHOULD 2 (M-2): pin WHICH shrunk pixels the analysis sees. The filter is
/// `image::imageops::thumbnail` from `image 0.24.9` (L-64); a different filter, rounding rule or
/// crate version gives a different buffer, so a different piece for every user, and turns this red.
/// The 100 % row pins that Full is the decoded buffer itself.
#[test]
fn should2_golden_shrunk_buffers() {
    let decoded = || {
        // Through the CLI's own decode (PNG bytes → RGB8), as `render::acquire_shrunk` sees it.
        audiohax::pure_analysis::load_pure_image_bytes(&png(&structured(203, 141))).expect("decode")
    };
    let golden: [(Shrink, (u32, u32), u64); 4] = [
        (Shrink::Full, (203, 141), GOLDEN_FULL),
        (Shrink::Percent75, (152, 106), GOLDEN_75),
        (Shrink::Percent50, (102, 71), GOLDEN_50),
        (Shrink::Percent25, (51, 35), GOLDEN_25),
    ];
    let mut got = Vec::new();
    let mut wrong = Vec::new();
    for (s, size, want) in golden {
        let out = apply(decoded(), s).expect("fits");
        assert_eq!((out.width(), out.height()), size, "{s:?} size");
        assert_eq!(shrunk_size(203, 141, s), size, "{s:?} arithmetic");
        let h = fnv64(out.as_rgb().as_raw());
        got.push(format!("{s:?}={h:#018x}"));
        if h != want {
            wrong.push(format!("{s:?}: {h:#018x} (golden {want:#018x})"));
        }
    }
    assert!(
        wrong.is_empty(),
        "the resize now makes different pixels, so every shrunk piece changes: {wrong:?}; all \
         measured: {got:?}"
    );
}
// Measured S125 lane 02S on the candidate-3 source (`image 0.24.9`, `imageops::thumbnail`). A
// filter-change mutation (`imageops::resize` Triangle) turns the three shrink rows red; Full stays.
const GOLDEN_FULL: u64 = 0x9fbc_7de8_4da7_9dd2;
const GOLDEN_75: u64 = 0x9890_2fb0_dce9_22cc;
const GOLDEN_50: u64 = 0xdb0b_58d8_bd74_cac0;
const GOLDEN_25: u64 = 0x2ad0_2154_23dd_0856;

/// SH5 S2/S3 + SK7 (library half): an infeasible level is refused with the typed error BEFORE the
/// resizer runs and before any note is reported; the picture is NOT refused without a shrink.
#[test]
fn infeasible_levels_are_refused_before_any_work() {
    let m = mappings();
    let (req, _) = cli_request(&["--seed", "7"]);
    for (w, h, s) in [
        (40, 30, Shrink::Percent50),
        (40, 30, Shrink::Percent25),
        (20, 20, Shrink::Percent75),
        (40, 16, Shrink::Percent75),
        (32768, 16, Shrink::Percent75),
        (16, 32768, Shrink::Percent25),
        (16, 16, Shrink::Percent75),
    ] {
        assert!(!fits(w, h, s));
        let bytes = png(&RgbImage::from_pixel(w, h, Rgb([90, 140, 200])));
        let before = resizes_on_this_thread();
        let mut notes = 0usize;
        let got = render_shrunk(
            &req,
            s,
            ImageInput::Bytes(&bytes),
            &m,
            SoundFontSource::Bundled,
            &mut |_| {
                notes += 1;
                Ok(())
            },
        );
        match got {
            Err(RenderError::Shrink(ShrinkError::TooSmall { percent, from, to })) => {
                assert_eq!(percent, s.percent());
                assert_eq!(from, (w, h));
                assert_eq!(to, shrunk_size(w, h, s));
                assert!(to.0 < SHRINK_MIN_SIDE || to.1 < SHRINK_MIN_SIDE);
            }
            other => panic!("{w}x{h} {s:?}: expected the floor refusal, got {other:?}"),
        }
        assert_eq!(resizes_on_this_thread(), before, "{w}x{h} {s:?} resized");
        assert_eq!(
            notes, 0,
            "{w}x{h} {s:?}: a note was reported before the refusal"
        );
    }
}

/// M-1 at the library seam: whatever the shrink, the analysed size is `shrunk_size`'s, on the
/// size §13 found an aspect-fitting resize gets wrong (71×78 at 75 % → 53×59, not 53×58).
#[test]
fn m1_the_analysed_size_is_the_printed_size() {
    for (w, h) in [(71u32, 78u32), (101, 100), (640, 427)] {
        for s in Shrink::ALL {
            if !fits(w, h, s) {
                continue;
            }
            let out = apply(PureImage::from_rgb(structured(w, h)), s).expect("apply");
            assert_eq!(
                (out.width(), out.height()),
                shrunk_size(w, h, s),
                "{w}x{h} {s:?}"
            );
        }
    }
}

// ── CLI ─────────────────────────────────────────────────────────────────────────────────────

/// SK2 / SH5 S5: exactly `100 75 50 25`; anything else is clap's possible-values error, exit 2,
/// listing the four. Nothing is read (no picture path is even valid here).
#[test]
fn sk2_the_cli_takes_exactly_the_four_levels() {
    for bad in [
        "60",
        "50%",
        "0",
        "-1",
        "1.5",
        "NaN",
        "99999999999",
        "full",
        "",
    ] {
        let o = run(
            &[
                "render",
                "--wav",
                "/nonexistent/x.wav",
                "--shrink",
                bad,
                "/nonexistent.png",
            ],
            30,
        );
        assert_eq!(o.rc, Some(2), "--shrink {bad:?}: {}", o.stderr);
        // clap reads `-1` as an unknown flag (its own error, still exit 2); every other value
        // gets the possible-values error that lists the four.
        if !bad.starts_with('-') {
            assert!(
                o.stderr.contains("[possible values: 100, 75, 50, 25]"),
                "--shrink {bad:?}: {}",
                o.stderr
            );
        }
    }
    for good in ["100", "75", "50", "25"] {
        assert!(
            Cli::try_parse_from(["audiohax", "render", "--shrink", good]).is_ok(),
            "{good}"
        );
        assert!(
            Cli::try_parse_from(["audiohax", "play", "--shrink", good]).is_ok(),
            "play {good}"
        );
    }
    // Not global: the other subcommands do not take it.
    assert!(Cli::try_parse_from(["audiohax", "analyze", "--shrink", "50"]).is_err());
    assert!(Cli::try_parse_from(["audiohax", "tui", "--shrink", "50"]).is_err());
}

/// SK7 / SH5 C-4: `render --wav out.wav --shrink 50 tiny40x30.png` exits non-zero with UI4's exact
/// message, writes no file and prints no notice. `play` refuses the same way, before any sound.
/// The same picture with no `--shrink` renders as today (the CLI has no general floor, OQ-2).
#[test]
fn sk7_the_cli_refuses_a_shrink_under_the_floor_and_nothing_else() {
    let d = scratch("sk7");
    let tiny = write_png(&d, "tiny40x30.png", &structured(40, 30));
    let wav = d.join("out.wav");
    let w = wav.display().to_string();
    let o = run(&["render", "--wav", &w, "--shrink", "50", &tiny], 60);
    assert_eq!(o.rc, Some(1), "{}", o.stderr);
    assert_eq!(
        o.stderr,
        "Error: --shrink 50 would make this picture 20x15 (from 40x30), and each side must stay \
         at least 16 px. Use a larger --shrink value, or leave --shrink off.\n"
    );
    assert!(!o.stderr.contains("Note:"));
    assert!(!wav.exists(), "a refused shrink wrote a WAV");
    let o = run(&["render", "--wav", &w, "--shrink", "25", &tiny], 60);
    assert!(
        o.stderr
            .contains("would make this picture 10x8 (from 40x30)"),
        "{}",
        o.stderr
    );
    // A picture already under 16 px: refused WITH a shrink, rendered as today WITHOUT one.
    let mini = write_png(&d, "mini8x6.png", &structured(8, 6));
    let o = run(&["render", "--wav", &w, "--shrink", "75", &mini], 60);
    assert_eq!(o.rc, Some(1));
    assert!(o
        .stderr
        .starts_with("Error: --shrink 75 would make this picture 6x5 (from 8x6)"));
    assert!(!wav.exists());
    // `play`: refused before the sink is built (nothing plays; the test has no audio device need).
    let o = run(&["play", "--shrink", "50", &tiny], 60);
    assert_eq!(o.rc, Some(1), "{}", o.stderr);
    assert!(o
        .stderr
        .contains("Error: --shrink 50 would make this picture 20x15"));
    assert!(
        !o.stdout.contains("Starting in-process synth"),
        "{}",
        o.stdout
    );
    // Without --shrink the 40×30 picture renders, as it did before S125.
    let o = run(&["render", "--wav", &w, &tiny], 120);
    assert_eq!(o.rc, Some(0), "{}", o.stderr);
    assert!(wav.exists());
    assert!(
        o.stderr.is_empty(),
        "no notice without --shrink: {}",
        o.stderr
    );
}

/// SK10 + SK1 (CLI half) + SK9 (CLI = library): the notice prints once, on stderr, only below 100;
/// stdout keeps its line count and every line's format (only numbers describing the piece move);
/// `--shrink 100` is byte-identical to no flag (WAV, stdout, stderr); and the CLI's WAV at each level
/// is the library's `render_shrunk` WAV, byte for byte.
#[test]
fn sk10_the_notice_is_stderr_only_and_the_default_is_unchanged() {
    let d = scratch("sk10");
    let pic = write_png(&d, "pic.png", &structured(160, 100));
    let wav = d.join("out.wav");
    let w = wav.display().to_string();
    let base = run(&["render", "--wav", &w, "--seed", "7", &pic], 120);
    assert_eq!(base.rc, Some(0), "{}", base.stderr);
    assert!(base.stderr.is_empty());
    let base_wav = std::fs::read(&wav).expect("wav");
    let o100 = run(
        &[
            "render", "--wav", &w, "--seed", "7", "--shrink", "100", &pic,
        ],
        120,
    );
    assert_eq!(o100.rc, Some(0));
    assert_eq!(o100.stdout, base.stdout, "--shrink 100 changed stdout");
    assert_eq!(o100.stderr, base.stderr, "--shrink 100 changed stderr");
    assert_eq!(
        std::fs::read(&wav).expect("wav"),
        base_wav,
        "--shrink 100 changed the WAV"
    );

    let bytes = std::fs::read(&pic).expect("pic");
    let (req, _) = cli_request(&["--seed", "7"]);
    for s in [Shrink::Percent75, Shrink::Percent50, Shrink::Percent25] {
        let pct = s.percent().to_string();
        let o = run(
            &["render", "--wav", &w, "--seed", "7", "--shrink", &pct, &pic],
            120,
        );
        assert_eq!(o.rc, Some(0), "{}", o.stderr);
        let (tw, th) = shrunk_size(160, 100, s);
        assert_eq!(
            o.stderr,
            format!(
                "Note: --shrink {pct} makes the music from a smaller copy of the picture \
                 ({tw}x{th}, from 160x100). Less detail means different music: this piece won't \
                 match the full-size one, and its notes, tempo and length can all change.\n"
            ),
            "exactly one notice line"
        );
        assert_eq!(
            digits_masked(&o.stdout),
            digits_masked(&base.stdout),
            "--shrink {pct}: stdout gained a line or changed a format"
        );
        // CLI == library, at this level (the WAV writer is the library's; compare the samples).
        let cli_wav = std::fs::read(&wav).expect("wav");
        assert_ne!(cli_wav, base_wav, "--shrink {pct} is inert on the CLI");
        let lib = lib_render(&req, s, &bytes);
        let lib_path = d.join(format!("lib-{pct}.wav"));
        audiohax::synth_sink::write_stereo_wav(&lib_path, lib.sample_rate, &lib.interleaved)
            .expect("write lib wav");
        assert_eq!(
            cli_wav,
            std::fs::read(&lib_path).expect("lib wav"),
            "--shrink {pct}: the CLI's WAV is not the library's"
        );
    }
}

/// SK11: more voices than the shrunk picture's smaller side gives the existing error, not a panic.
#[test]
fn sk11_too_many_voices_after_a_shrink_is_an_error_not_a_panic() {
    let d = scratch("sk11");
    // 64×40 at 50 % → 32×20: 24 voices is more than the 20-px side can supply.
    let pic = write_png(&d, "pic.png", &structured(64, 40));
    let w = d.join("out.wav").display().to_string();
    let o = run(
        &[
            "render",
            "--wav",
            &w,
            "--shrink",
            "50",
            "--instruments",
            "24",
            &pic,
        ],
        120,
    );
    assert_eq!(o.rc, Some(1), "stdout {} stderr {}", o.stdout, o.stderr);
    assert!(!o.stderr.contains("panicked"), "{}", o.stderr);
    assert!(o.stderr.starts_with("Note: --shrink 50"), "{}", o.stderr);
    assert!(o.stderr.contains("\nError: "), "{}", o.stderr);
}

/// SK12: `render --help` and `play --help` each carry UI4's doc text and clap's two suffixes, and
/// the floor it states is the library's constant. (The full diff against the BEFORE binary is in
/// the build report; this pins the text.)
#[test]
fn sk12_both_helps_carry_the_flag_text() {
    let want = "Make the music from a smaller copy of the picture: 75, 50 or 25 (percent of each \
                side), or 100 for the whole picture (the default). A smaller picture takes less \
                time to turn into music, but it has less detail, so the music is different: the \
                piece won't match the full-size one, and its notes, tempo and length can all \
                change. Each side must stay at least 16 px; a shrink that would go below that is \
                refused";
    assert!(want.contains(&format!("at least {SHRINK_MIN_SIDE} px;")));
    for sub in ["render", "play"] {
        let o = run(&[sub, "--help"], 30);
        assert_eq!(o.rc, Some(0));
        let flat: String = o.stdout.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains("--shrink <PERCENT>"), "{sub}: {}", o.stdout);
        assert!(
            flat.contains(want),
            "{sub} --help lacks the doc text:\n{}",
            o.stdout
        );
        assert!(flat.contains("[default: 100]"), "{sub}");
        assert!(flat.contains("[possible values: 100, 75, 50, 25]"), "{sub}");
    }
    let o = run(&["analyze", "--help"], 30);
    assert!(
        !o.stdout.contains("--shrink"),
        "analyze --help gained --shrink"
    );
}

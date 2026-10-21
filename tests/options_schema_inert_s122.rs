//! tests/options_schema_inert_s122.rs — S122 Phase 1, FORCING TEST 2: the inert-knob guard
//! (`s121-01` § 5.3 / § 3.5).
//!
//! For EVERY `Everyday` / `Advanced` row of `audiohax::options_schema::GUI_TIERS`, render one
//! fixture with the option at its default and again at a non-default value, and FAIL if the WAV
//! bytes are identical. A knob that stops doing anything turns the suite red instead of shipping as
//! a dead slider. `s121-01` § 3.5 measured two such knobs on `render` (`--ms-per-step`,
//! `--jitter-percent`); they are `Hidden` for that reason, and promoting either one makes THIS test
//! fail until the knob is made live.
//!
//! The test is driven by the schema, not by a hand-kept list of options: it walks
//! `render_options_schema()`, so a newly promoted row is covered automatically. The one thing it
//! needs per row is a NON-DEFAULT probe value, in [`PROBES`] below; a visible row without one fails
//! with the exact line to add.
//!
//! HOW EACH RENDER IS BUILT. Each request is parsed from CLI tokens by clap (`Cli::try_parse_from`)
//! and adapted with the CLI's own `RenderRequest::from(&RenderArgs)`, so the probe exercises the
//! same path `audiohax render … --wav` does (minus file I/O). A row's `requires` (e.g.
//! `expression=on` for `v2`) are applied to BOTH sides, so the comparison isolates the one option.
//! The second test checks that each `requires` entry mirrors a REAL refusal: violating it must make
//! `render()` return `RenderError::Refused`.
//!
//! `--soundfont` needs a second, different SoundFont, and no `.sf2` is tracked in git. The test
//! derives one from the embedded font's source file (`assets/soundfonts/default.sf2`, which must
//! exist for the crate to compile at all): every 16-bit sample in the `sdta/smpl` chunk is negated.
//! The result is a valid SF2 with identical structure and inverted waveforms, so a render through it
//! differs iff `--soundfont` actually reaches the synthesizer.
//!
//! Fixture: `assets/images/Lena.png` (the only committed PNG; JPEG decode differs across CPUs,
//! `s121-01` § 7 U1). Comparisons are same-process, same-machine, so they are portable. Cost: about
//! a dozen debug renders, run on parallel threads (each render sets thread-local registers, so each
//! runs on its own thread, as `tests/render_golden_s122.rs` does).
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use clap::Parser;

use audiohax::cli::{Cli, Command};
use audiohax::mapping_loader::load_mappings;
use audiohax::options_schema::{render_options_schema, OptionSpec, Tier};
use audiohax::render::{render, ImageInput, RenderError, RenderRequest};
use audiohax::synth_sink::{write_stereo_wav, SoundFontSource};

/// One NON-DEFAULT probe per visible row: the CLI tokens that set the option away from its
/// default. `@ALT_SF2@` is replaced by the derived SoundFont's path. Add a line here when you
/// promote a row to `Everyday` / `Advanced`.
const PROBES: &[(&str, &[&str])] = &[
    ("seed", &["--seed", "7"]),
    ("instruments", &["--instruments", "6"]),
    ("reverb", &["--reverb", "off"]),
    ("gain", &["--gain", "0.8"]),
    ("expression", &["--expression", "on"]),
    ("v2", &["--v2", "all"]),
    ("thickness", &["--thickness", "0.3"]),
    ("steps", &["--steps", "10"]),
    ("loop", &["--loop"]),
    ("soundfont", &["--soundfont", "@ALT_SF2@"]),
];

/// Tokens every render carries to keep the test fast, EXCEPT when the option under test is the one
/// these tokens set (then both sides use that option's true default). `--steps 6` scans 6 bars
/// instead of 40; the piece is still 36 steps long (the plan sets the length, `s121-01` § 3.5), as in
/// `tests/render_golden_s122.rs`.
const FAST_BASE: (&str, &[&str]) = ("steps", &["--steps", "6"]);

/// For each `requires` entry, the CLI tokens that VIOLATE it (used by the refusal-mirror test).
const VIOLATIONS: &[(&str, &[&str])] = &[
    ("expression=on", &["--expression", "off"]),
    ("gain=1", &["--gain", "0.8"]),
];

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// The derived second SoundFont (sample-negated copy of the embedded one), written once.
fn alt_sf2() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest("assets/soundfonts/default.sf2");
        let mut bytes =
            std::fs::read(&src).expect("assets/soundfonts/default.sf2 (needed to build)");
        let (off, len) = find_smpl(&bytes).expect("default.sf2 has an sdta/smpl chunk");
        for pair in bytes[off..off + (len & !1)].chunks_exact_mut(2) {
            let v = i16::from_le_bytes([pair[0], pair[1]]).wrapping_neg();
            pair.copy_from_slice(&v.to_le_bytes());
        }
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "options_schema_inert_s122_alt_{}.sf2",
            std::process::id()
        ));
        std::fs::write(&out, &bytes).expect("write derived SoundFont");
        out
    })
}

/// Locate the `smpl` sub-chunk of the `LIST sdta` chunk in a RIFF `sfbk` file: `(data offset, len)`.
fn find_smpl(b: &[u8]) -> Option<(usize, usize)> {
    let u32le = |i: usize| -> Option<usize> {
        Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize)
    };
    if b.get(0..4)? != b"RIFF" || b.get(8..12)? != b"sfbk" {
        return None;
    }
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = b.get(i..i + 4)?;
        let size = u32le(i + 4)?;
        if id == b"LIST" && b.get(i + 8..i + 12)? == b"sdta" {
            let end = i + 8 + size;
            let mut j = i + 12;
            while j + 8 <= end {
                let sid = b.get(j..j + 4)?;
                let ssize = u32le(j + 4)?;
                if sid == b"smpl" {
                    return Some((j + 8, ssize));
                }
                j += 8 + ssize + (ssize & 1);
            }
            return None;
        }
        i += 8 + size + (size & 1);
    }
    None
}

/// The baseline tokens for testing option `o`: the fast base (unless `o` is the option it sets),
/// then each `requires` entry `"<id>=<value>"` as `["--<id>", "<value>"]`, skipping an entry whose
/// value already IS that option's default (it would render the same request twice).
fn base_tokens(o: &OptionSpec, schema: &[OptionSpec]) -> Vec<String> {
    let mut t: Vec<String> = Vec::new();
    if o.id != FAST_BASE.0 {
        t.extend(FAST_BASE.1.iter().map(|s| s.to_string()));
    }
    for r in &o.requires {
        let (id, v) = r.split_once('=').expect("requires is id=value");
        let default = schema
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.default.clone());
        let is_default = default.as_deref().is_some_and(|d| {
            d == v || matches!((d.parse::<f64>(), v.parse::<f64>()), (Ok(a), Ok(b)) if a == b)
        });
        if !is_default {
            t.push(format!("--{id}"));
            t.push(v.to_string());
        }
    }
    t
}

fn probe_tokens(id: &str) -> Option<Vec<String>> {
    PROBES.iter().find(|(p, _)| *p == id).map(|(_, toks)| {
        toks.iter()
            .map(|t| {
                if *t == "@ALT_SF2@" {
                    alt_sf2().display().to_string()
                } else {
                    t.to_string()
                }
            })
            .collect()
    })
}

/// Parse `audiohax render <Lena> --wav <unused> <extra…>` with the real grammar.
fn parse(extra: &[String]) -> audiohax::cli::RenderArgs {
    let lena = manifest("assets/images/Lena.png");
    let mut argv: Vec<String> = vec![
        "audiohax".into(),
        "render".into(),
        lena.display().to_string(),
        "--wav".into(),
        "unused.wav".into(),
    ];
    argv.extend(extra.iter().cloned());
    match Cli::try_parse_from(&argv) {
        Ok(Cli {
            command: Command::Render(r),
            ..
        }) => r,
        Ok(_) => panic!("{argv:?} did not parse as `render`"),
        Err(e) => panic!("{argv:?} does not parse: {e}"),
    }
}

/// Render the parsed args through the library exactly as the CLI adapter does, on a NEW thread
/// (thread-local registers). Returns the WAV file bytes, or the render error.
fn render_wav(extra: Vec<String>) -> std::thread::JoinHandle<Result<Vec<u8>, RenderError>> {
    std::thread::spawn(move || {
        let args = parse(&extra);
        let req = RenderRequest::from(&args);
        let mappings = load_mappings(manifest("assets/mappings.json").to_str().expect("utf-8"))
            .expect("assets/mappings.json loads");
        let image = args.image.clone().expect("image given");
        let font = match &args.audio.soundfont {
            Some(p) => SoundFontSource::Path(p.as_path()),
            None => SoundFontSource::Bundled,
        };
        let out = render(&req, ImageInput::Path(&image), &mappings, font, &mut |_| {
            Ok(())
        })?;
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "options_schema_inert_s122_{}_{:?}.wav",
            std::process::id(),
            std::thread::current().id()
        ));
        write_stereo_wav(&path, out.sample_rate, &out.interleaved).expect("write WAV");
        let bytes = std::fs::read(&path).expect("read WAV back");
        let _ = std::fs::remove_file(&path);
        Ok(bytes)
    })
}

fn visible(schema: &[OptionSpec]) -> Vec<OptionSpec> {
    schema
        .iter()
        .filter(|o| matches!(o.tier, Tier::Everyday | Tier::Advanced))
        .cloned()
        .collect()
}

/// FORCING TEST 2. Every visible option changes the rendered bytes.
#[test]
fn every_visible_option_changes_the_render() {
    let schema = render_options_schema().expect("schema builds (forcing test 1 owns this)");
    let rows = visible(&schema);
    assert!(
        !rows.is_empty(),
        "no visible rows: the guard would be vacuous"
    );

    // Structural failures first, with the line to add.
    let mut problems = Vec::new();
    for (p, _) in PROBES {
        if !schema.iter().any(|o| o.id == *p) {
            problems.push(format!(
                "PROBES names `{p}`, which the render schema no longer has: delete its line in \
                 tests/options_schema_inert_s122.rs"
            ));
        }
    }
    for o in &rows {
        match probe_tokens(&o.id) {
            None => problems.push(format!(
                "`{}` is {:?} but has no probe. Add ONE line to PROBES in \
                 tests/options_schema_inert_s122.rs:\n    (\"{}\", &[\"--{}\", \"<a non-default value>\"]),",
                o.id, o.tier, o.id, o.id
            )),
            Some(t) => {
                if t.len() == 2 && o.default.as_deref() == Some(t[1].as_str()) {
                    problems.push(format!(
                        "probe for `{}` uses its DEFAULT value {}: pick a non-default one",
                        o.id, t[1]
                    ));
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "\n\nINERT-KNOB FORCING TEST 2 (setup)\n{}\n",
        problems.join("\n")
    );

    // Every distinct render, once, in parallel. Key = the exact token list after `requires`.
    let mut jobs: HashMap<Vec<String>, std::thread::JoinHandle<Result<Vec<u8>, RenderError>>> =
        HashMap::new();
    let mut pairs = Vec::new();
    for o in &rows {
        let base = base_tokens(o, &schema);
        let mut probe = base.clone();
        probe.extend(probe_tokens(&o.id).expect("checked above"));
        for k in [&base, &probe] {
            if !jobs.contains_key(k) {
                jobs.insert(k.clone(), render_wav(k.clone()));
            }
        }
        pairs.push((o.clone(), base, probe));
    }
    let results: HashMap<Vec<String>, Result<Vec<u8>, String>> = jobs
        .into_iter()
        .map(|(k, h)| {
            let r = h
                .join()
                .unwrap_or_else(|_| panic!("render thread for {k:?} panicked"))
                .map_err(|e| format!("{e:?}"));
            (k, r)
        })
        .collect();

    let mut dead = Vec::new();
    for (o, base, probe) in &pairs {
        // A render failure is reported with the rest, not as a first-failure panic, so one run
        // shows every problem. A probe that REFUSES usually means a missing `requires` entry.
        let (b, p) = match (&results[base], &results[probe]) {
            (Ok(b), Ok(p)) => (b, p),
            (Err(e), _) => {
                dead.push(format!(
                    "`{}` baseline {base:?} failed to render: {e}",
                    o.id
                ));
                continue;
            }
            (_, Err(e)) => {
                dead.push(format!(
                    "`{}` probe {probe:?} failed to render (missing `requires` in its GUI_TIERS \
                     row?): {e}",
                    o.id
                ));
                continue;
            }
        };
        if b == p {
            dead.push(format!(
                "`{}` ({:?}, label {:?}): rendering with {probe:?} is BYTE-IDENTICAL to {base:?}. \
                 The option does nothing on `render`, so the GUI would ship a dead control. Set its \
                 GUI_TIERS row to Tier::Hidden with the measured reason, or make the option live.",
                o.id, o.tier, o.label
            ));
        }
    }
    assert!(
        dead.is_empty(),
        "\n\nINERT-KNOB FORCING TEST 2 FAILED\n{}\n",
        dead.join("\n")
    );
}

/// Each `requires` entry mirrors a REAL refusal: violating it while setting the option must make
/// `render()` refuse. A precondition the renderer does not enforce would make the GUI disable a
/// control for no reason; one the renderer enforces but the schema omits is caught by forcing test
/// 2 (the probe would fail to render).
#[test]
fn every_requires_mirrors_a_real_refusal() {
    let schema = render_options_schema().expect("schema builds");
    let mut jobs = Vec::new();
    for o in schema.iter().filter(|o| !o.requires.is_empty()) {
        for req in &o.requires {
            let viol = VIOLATIONS
                .iter()
                .find(|(r, _)| r == req)
                .unwrap_or_else(|| {
                    panic!(
                        "`{}` requires `{req}` but VIOLATIONS has no entry. Add ONE line to \
                         VIOLATIONS in tests/options_schema_inert_s122.rs:\n    (\"{req}\", \
                         &[\"--<id>\", \"<a value that violates it>\"]),",
                        o.id
                    )
                })
                .1;
            let mut toks: Vec<String> = FAST_BASE.1.iter().map(|s| s.to_string()).collect();
            toks.extend(viol.iter().map(|s| s.to_string()));
            toks.extend(
                probe_tokens(&o.id)
                    .unwrap_or_else(|| panic!("`{}` has requires but no probe in PROBES", o.id)),
            );
            jobs.push((o.id.clone(), req.clone(), toks.clone(), render_wav(toks)));
        }
    }
    assert!(
        !jobs.is_empty(),
        "no requires in the schema: this test would be vacuous"
    );
    for (id, req, toks, h) in jobs {
        match h.join().expect("render thread") {
            Err(RenderError::Refused(_)) => {}
            Err(e) => panic!(
                "`{id}` with `{req}` violated ({toks:?}) failed, but not as a refusal: {e:?}"
            ),
            Ok(_) => panic!(
                "`{id}` declares requires `{req}`, but rendering with it violated ({toks:?}) \
                 SUCCEEDED. The schema claims a precondition the renderer does not enforce."
            ),
        }
    }
}

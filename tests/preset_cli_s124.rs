//! S124 Test Engineer (Specialist 5): the shipped `audiohax` and `audiohax-tui` BINARIES under the
//! preset slice — mappings resolution (L-29), the bounded loader at the cwd path and at
//! `--mappings` (L-32), the 600 s piece cap (MB-5 / L-41), the flag conflict (L-30) and the
//! refusal order (§1 T-12) — plus the library-level precedence (L-30) and composition of the
//! shapes the loader admits.
//!
//! WHY BINARY-LEVEL for resolution: which file a run reads depends on the PROCESS cwd, and a test
//! that changed its own cwd would race every other test thread. Each case here runs the real binary
//! (`CARGO_BIN_EXE_audiohax`) in its own private directory.
//!
//! CONTROLS: the harness stages a mappings file byte-equal to the embedded one, so "identical"
//! alone cannot tell a cwd read from an embedded one (§1 H2). Every identity claim below is paired
//! with a run that MUST differ (a perturbed cwd table), and the test fails if that control does not
//! fire.
//!
//! Every child runs behind a wall-clock guard (killed and failed, never hung). Scratch lives under
//! `CARGO_TARGET_TMPDIR` (inside the target dir), never `/tmp`.
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use audiohax::mapping_loader::DEFAULT_MAPPINGS_JSON;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_audiohax");
const TUI: &str = env!("CARGO_BIN_EXE_audiohax-tui");
/// The smallest tracked picture (fastest render; `render_cli_determinism_s55.rs` uses it too).
const IMG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/images/AudioHaxImg1.jpg"
);

fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-cli-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

struct Out {
    rc: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run `exe args…` in `cwd` with stdin closed; kill and FAIL after `secs`.
fn run_in(exe: &str, cwd: &Path, args: &[&str], secs: u64) -> Out {
    let mut child = Command::new(exe)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let t0 = Instant::now();
    loop {
        if child.try_wait().expect("wait").is_some() {
            break;
        }
        if t0.elapsed() > Duration::from_secs(secs) {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "{exe} {args:?} in {}: still running after {secs} s (hang)",
                cwd.display()
            );
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

fn run(cwd: &Path, args: &[&str]) -> Out {
    run_in(BIN, cwd, args, 240)
}

fn shipped() -> Value {
    serde_json::from_str(DEFAULT_MAPPINGS_JSON).expect("json")
}

/// Plant `body` as `cwd/assets/mappings.json`.
fn plant(cwd: &Path, body: &str) -> PathBuf {
    std::fs::create_dir_all(cwd.join("assets")).expect("assets dir");
    let p = cwd.join("assets/mappings.json");
    std::fs::write(&p, body).expect("plant");
    p
}

/// The shipped table with every MIDI program (ensembles and pools) set to 19 — a table that
/// passes every bound and sounds different on every picture (the CONTROL table).
fn perturbed() -> String {
    let mut v = shipped();
    for e in v["composition"]["ensemble_catalogue"]
        .as_array_mut()
        .expect("ensembles")
    {
        for p in e["programs"].as_array_mut().expect("programs") {
            *p = json!(19);
        }
    }
    for r in v["composition"]["instrumentation_pools"]["roles"]
        .as_array_mut()
        .expect("roles")
    {
        for e in r["entries"].as_array_mut().expect("entries") {
            e["program"] = json!(19);
        }
    }
    serde_json::to_string(&v).expect("ser")
}

/// Render IMG in `cwd` to `cwd/out.wav` + `cwd/out.json` with `extra` flags; return
/// (rc, stdout, stderr, wav bytes, dump bytes). Relative output names keep stdout comparable.
fn render5(cwd: &Path, extra: &[&str]) -> (Option<i32>, String, String, Vec<u8>, Vec<u8>) {
    let _ = std::fs::remove_file(cwd.join("out.wav"));
    let _ = std::fs::remove_file(cwd.join("out.json"));
    let mut args = vec![
        "render",
        IMG,
        "--wav",
        "out.wav",
        "--seed",
        "7",
        "--steps",
        "6",
        "--event-dump",
        "out.json",
    ];
    args.extend_from_slice(extra);
    let o = run(cwd, &args);
    let wav = std::fs::read(cwd.join("out.wav")).unwrap_or_default();
    let dump = std::fs::read(cwd.join("out.json")).unwrap_or_default();
    (o.rc, o.stdout, o.stderr, wav, dump)
}

fn assert_no_debug_text(s: &str) {
    for bad in ["Os {", "Kind(", "kind:"] {
        assert!(!s.contains(bad), "Rust Debug text `{bad}` on stderr: {s}");
    }
}

// ── L-29: cwd first, embedded fallback; --preset default = embedded; --mappings FILE = that file ─

/// L-29 (§1 option B) with controls that can fail:
/// * a cwd holding the SHIPPED table and an EMPTY cwd render identically on all five channels
///   (rc, stdout, stderr, WAV, dump) — the embedded fallback is the shipped table;
/// * CONTROL: a cwd holding a PERTURBED table renders DIFFERENTLY with no flag (the cwd file is
///   still read). If this control does not fire, nothing above or below means anything;
/// * from that perturbed cwd, `--preset default` renders the shipped piece (embedded, cwd ignored)
///   and `--mappings <shipped file>` renders the shipped piece (explicit file wins);
/// * from the EMPTY cwd, `--mappings <perturbed file>` renders the perturbed piece (the explicit
///   file is really read, not the embedded table);
/// * L-29's own sentence: `--preset default` equals the no-flag render byte-for-byte, all five
///   channels, in a cwd holding the shipped mappings.
#[test]
fn l29_resolution_order_with_controls_that_can_fail() {
    let shipped_cwd = scratch("l29-shipped");
    let shipped_file = plant(&shipped_cwd, DEFAULT_MAPPINGS_JSON);
    let empty_cwd = scratch("l29-empty");
    let pert_cwd = scratch("l29-pert");
    let pert_file = plant(&pert_cwd, &perturbed());

    let a = render5(&shipped_cwd, &[]);
    assert_eq!(a.0, Some(0), "shipped cwd renders: {}", a.2);
    assert!(!a.3.is_empty() && !a.4.is_empty(), "WAV and dump written");

    let b = render5(&empty_cwd, &[]);
    assert_eq!(
        b.0,
        Some(0),
        "empty cwd renders from the embedded table: {}",
        b.2
    );
    assert_eq!((&b.1, &b.2), (&a.1, &a.2), "stdout/stderr equal");
    assert!(
        b.3 == a.3 && b.4 == a.4,
        "embedded fallback == shipped cwd file (WAV and dump)"
    );

    let c = render5(&pert_cwd, &[]);
    assert_eq!(c.0, Some(0), "{}", c.2);
    assert!(
        c.3 != a.3,
        "CONTROL DID NOT FIRE: a perturbed cwd table rendered the shipped piece — either the cwd \
         file is not read (L-29 broken) or the perturbation is inaudible; every identity in this \
         test is then meaningless"
    );

    // (Measured: the event dump carries no MIDI program numbers, so a programs-only perturbation
    // leaves the DUMP identical and changes the WAV; the WAV is the channel that can see it.)

    let c_preset = render5(&pert_cwd, &["--preset", "default"]);
    assert_eq!(c_preset.0, Some(0), "{}", c_preset.2);
    assert!(
        c_preset.3 == a.3 && c_preset.4 == a.4,
        "--preset default must use the EMBEDDED table and ignore the cwd file"
    );

    let shipped_abs = shipped_file.to_str().expect("utf8");
    let c_map = render5(&pert_cwd, &["--mappings", shipped_abs]);
    assert_eq!(c_map.0, Some(0), "{}", c_map.2);
    assert!(c_map.3 == a.3, "--mappings FILE beats the cwd file");

    let pert_abs = pert_file.to_str().expect("utf8");
    let b_map = render5(&empty_cwd, &["--mappings", pert_abs]);
    assert_eq!(b_map.0, Some(0), "{}", b_map.2);
    assert!(
        b_map.3 == c.3,
        "--mappings FILE is really read (perturbed piece from an empty cwd)"
    );

    let a_preset = render5(&shipped_cwd, &["--preset", "default"]);
    assert_eq!(a_preset.0, a.0);
    assert_eq!(
        a_preset.1, a.1,
        "L-29/LD-4: --preset prints nothing new (stdout)"
    );
    assert_eq!(a_preset.2, a.2, "stderr");
    assert!(
        a_preset.3 == a.3 && a_preset.4 == a.4,
        "WAV and dump byte-identical"
    );
}

// ── H12 under L-29: a hostile cwd file is READ, and refused loudly ──────────────────────────

/// the security review's H12 was written for the drop-the-cwd-read option; under L-29 the cwd file IS read,
/// so the owed property is that a hostile one is REFUSED, naming the file and the field, before
/// anything renders or any port opens — on `render`, on `play`, and in the TUI bin — while
/// `--preset default` from the same cwd still renders.
#[test]
fn h12_a_hostile_cwd_table_is_refused_on_every_front_end() {
    let cwd = scratch("h12");
    let mut v = shipped();
    v["composition"]["affect"]["character_tempo"]["march"] = json!({"bpm_min": 200, "bpm_max": 50});
    plant(&cwd, &serde_json::to_string(&v).expect("ser"));
    let want =
        "The mappings file `assets/mappings.json`: `composition.affect.character_tempo.march` \
                has bpm_min (200) greater than bpm_max (50). Nothing was rendered.";

    let o = run(&cwd, &["render", IMG, "--wav", "x.wav", "--seed", "7"]);
    assert_eq!(o.rc, Some(1), "render: {}", o.stderr);
    assert!(o.stderr.contains(want), "render stderr: {}", o.stderr);
    assert!(!cwd.join("x.wav").exists(), "no WAV written");
    assert_no_debug_text(&o.stderr);

    let o = run(&cwd, &["play", IMG, "--output", "midi", "--seed", "7"]);
    assert_eq!(o.rc, Some(1), "play: {}", o.stderr);
    assert!(o.stderr.contains(want), "play stderr: {}", o.stderr);

    let o = run_in(TUI, &cwd, &[], 60);
    assert_ne!(o.rc, Some(0), "tui must refuse: {}", o.stderr);
    assert!(
        o.stderr
            .contains("The mappings file `assets/mappings.json`")
            && o.stderr
                .contains("composition.affect.character_tempo.march"),
        "tui stderr: {}",
        o.stderr
    );

    let o = run(
        &cwd,
        &[
            "render", IMG, "--wav", "ok.wav", "--seed", "7", "--steps", "6", "--preset", "default",
        ],
    );
    assert_eq!(
        o.rc,
        Some(0),
        "--preset default escapes the hostile cwd file: {}",
        o.stderr
    );
    assert!(cwd.join("ok.wav").exists());
}

// ── special files: never hang ───────────────────────────────────────────────────────────────

/// H8 at the BINARY: a FIFO planted at `./assets/mappings.json`, a FIFO given as `--mappings`, a
/// directory at the cwd path, and `/dev/zero` as `--mappings` are each refused promptly with the
/// MB-1 sentence (each child is killed and the test failed if it runs 30 s).
#[cfg(unix)]
#[test]
fn h8_special_files_are_refused_promptly_by_the_binary() {
    let fifo_cwd = scratch("fifo-cwd");
    std::fs::create_dir_all(fifo_cwd.join("assets")).expect("assets");
    let fifo = fifo_cwd.join("assets/mappings.json");
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo")
        .success());

    let o = run_in(BIN, &fifo_cwd, &["render", IMG, "--wav", "x.wav"], 30);
    assert_eq!(o.rc, Some(1), "{}", o.stderr);
    assert!(
        o.stderr
            .contains("The mappings file `assets/mappings.json` is not a regular file"),
        "{}",
        o.stderr
    );
    let o = run_in(BIN, &fifo_cwd, &["play", IMG, "--output", "midi"], 30);
    assert_eq!(o.rc, Some(1), "play: {}", o.stderr);
    assert!(o.stderr.contains("is not a regular file"), "{}", o.stderr);
    let o = run_in(TUI, &fifo_cwd, &[], 30);
    assert_ne!(o.rc, Some(0));
    assert!(
        o.stderr.contains("is not a regular file"),
        "tui: {}",
        o.stderr
    );

    let empty = scratch("fifo-flag");
    let fifo_s = fifo.to_str().expect("utf8");
    let o = run_in(
        BIN,
        &empty,
        &["render", IMG, "--wav", "x.wav", "--mappings", fifo_s],
        30,
    );
    assert_eq!(o.rc, Some(1));
    assert!(o.stderr.contains("is not a regular file"), "{}", o.stderr);
    let o = run_in(
        BIN,
        &empty,
        &["render", IMG, "--wav", "x.wav", "--mappings", "/dev/zero"],
        30,
    );
    assert_eq!(o.rc, Some(1));
    assert!(
        o.stderr.contains("`/dev/zero` is not a regular file"),
        "{}",
        o.stderr
    );

    let dir_cwd = scratch("dir-cwd");
    std::fs::create_dir_all(dir_cwd.join("assets/mappings.json")).expect("dir at the path");
    let o = run_in(BIN, &dir_cwd, &["render", IMG, "--wav", "x.wav"], 30);
    assert_eq!(
        o.rc,
        Some(1),
        "a directory at the cwd path is reported, not replaced"
    );
    assert!(o.stderr.contains("is not a regular file"), "{}", o.stderr);

    let o = run_in(
        BIN,
        &empty,
        &["render", IMG, "--wav", "x.wav", "--mappings", "nope.json"],
        30,
    );
    assert_eq!(o.rc, Some(1));
    assert!(
        o.stderr
            .contains("The mappings file `nope.json` could not be read: No such file or directory"),
        "{}",
        o.stderr
    );
    assert_no_debug_text(&o.stderr);
}

/// L-29's doc on `resolve_cli_mappings` says only `NotFound` falls back, "so an unreadable or
/// non-regular file there is reported rather than silently replaced by the built-in table". A
/// DANGLING SYMLINK at `./assets/mappings.json` is present-but-broken, yet `fs::metadata` follows
/// it and reports `NotFound`, so the run silently uses the built-in table.
#[cfg(unix)]
#[test]
fn f3_a_dangling_symlink_at_the_cwd_path_is_reported_not_replaced() {
    let cwd = scratch("dangling");
    std::fs::create_dir_all(cwd.join("assets")).expect("assets");
    std::os::unix::fs::symlink(
        cwd.join("assets/edited-mappings.json"),
        cwd.join("assets/mappings.json"),
    )
    .expect("symlink");
    let o = run(
        &cwd,
        &[
            "render", IMG, "--wav", "x.wav", "--seed", "7", "--steps", "6",
        ],
    );
    assert_eq!(
        o.rc,
        Some(1),
        "a broken link the user put there must be reported, not replaced by the built-in table \
         (rc={:?}, stdout={})",
        o.rc,
        o.stdout
    );
}

// ── MB-5 / L-41: the 600 s cap, the 1 BPM floor ─────────────────────────────────────────────

/// Every tempo field of the shipped table set to `bpm`.
fn all_tempi(bpm: f64) -> String {
    let mut v = shipped();
    v["composition"]["affect"]["energy_tempo"] = json!({"bpm_at_zero": bpm, "bpm_at_one": bpm});
    for (_, w) in v["composition"]["affect"]["character_tempo"]
        .as_object_mut()
        .expect("rails")
    {
        *w = json!({"bpm_min": bpm, "bpm_max": bpm});
    }
    for (_, t) in v["global"]["brightness_to_tempo_bpm"]
        .as_object_mut()
        .expect("legacy")
    {
        *t = json!(bpm.max(1.0) as u64);
    }
    serde_json::to_string(&v).expect("ser")
}

/// MB-5 + L-41: a table at exactly the 1 BPM floor PASSES the validator (the floor is inclusive)
/// and is then refused by the 600 s piece cap — before synthesis (prompt, a normal exit, no WAV,
/// no multi-GB allocation) — and the refusal names the mappings file and the tempo/length fields
/// (L-41's "owed if cheap"). Same on `play` (the prepare path), and with the file planted in the
/// cwd (named as `assets/mappings.json`). A table at 0.5 BPM gets the VALIDATOR's sentence
/// instead (the floor is a refusal, not a clamp).
#[test]
fn mb5_one_bpm_passes_the_floor_and_is_refused_by_the_600_s_cap() {
    let d = scratch("cap");
    std::fs::write(d.join("slow1.json"), all_tempi(1.0)).expect("write");
    let t0 = Instant::now();
    let o = run(
        &d,
        &[
            "render",
            IMG,
            "--wav",
            "slow.wav",
            "--seed",
            "7",
            "--mappings",
            "slow1.json",
        ],
    );
    let took = t0.elapsed();
    assert_eq!(
        o.rc,
        Some(1),
        "refused with a normal exit (not a signal): {}",
        o.stderr
    );
    for want in [
        "This piece would last",
        "the limit is 10 minutes. Nothing was rendered.",
        "The mappings file `slow1.json` sets this piece's tempo",
        "composition.affect.energy_tempo",
        "composition.affect.character_tempo",
        "global.brightness_to_tempo_bpm",
        "composition.form_catalogue[].sections",
    ] {
        assert!(o.stderr.contains(want), "missing {want:?} in: {}", o.stderr);
    }
    assert!(!d.join("slow.wav").exists(), "no WAV");
    assert!(
        took < Duration::from_secs(120),
        "refused before synthesis: {took:?}"
    );

    let o = run(
        &d,
        &[
            "play",
            IMG,
            "--output",
            "midi",
            "--seed",
            "7",
            "--mappings",
            "slow1.json",
        ],
    );
    assert_eq!(o.rc, Some(1), "play: {}", o.stderr);
    assert!(
        o.stderr.contains("the limit is 10 minutes"),
        "play: {}",
        o.stderr
    );

    let cwd = scratch("cap-cwd");
    plant(&cwd, &all_tempi(1.0));
    let o = run(&cwd, &["render", IMG, "--wav", "slow.wav", "--seed", "7"]);
    assert_eq!(o.rc, Some(1));
    assert!(
        o.stderr
            .contains("The mappings file `assets/mappings.json` sets this piece's tempo"),
        "{}",
        o.stderr
    );

    std::fs::write(d.join("half.json"), all_tempi(0.5)).expect("write");
    let o = run(
        &d,
        &[
            "render",
            IMG,
            "--wav",
            "half.wav",
            "--mappings",
            "half.json",
        ],
    );
    assert_eq!(o.rc, Some(1));
    assert!(
        o.stderr.contains("The mappings file `half.json`: `")
            && o.stderr
                .contains("is 0.5 BPM; a tempo must be between 1 and 400 BPM"),
        "the floor refuses 0.5 by name: {}",
        o.stderr
    );
    assert!(!o.stderr.contains("This piece would last"), "{}", o.stderr);
}

// ── L-30 at the binary; §1 T-12 order ───────────────────────────────────────────────────────

/// L-30 / T-9 at the binary: `--preset default --mappings x` exits 2 with clap's conflict message
/// and writes nothing; an unknown preset exits 2 listing the possible values; §1 T-12: with
/// `--loop --gain 0.5 --mappings missing.json` the LOOP refusal fires first (validate before the
/// mappings read), so the order of refusals is unchanged.
#[test]
fn conflict_unknown_preset_and_refusal_order_at_the_binary() {
    let d = scratch("conflict");
    let o = run(
        &d,
        &[
            "render",
            IMG,
            "--wav",
            "x.wav",
            "--preset",
            "default",
            "--mappings",
            "m.json",
        ],
    );
    assert_eq!(o.rc, Some(2), "{}", o.stderr);
    assert!(
        o.stderr
            .contains("the argument '--preset <NAME>' cannot be used with '--mappings <FILE>'"),
        "{}",
        o.stderr
    );
    assert!(!d.join("x.wav").exists());

    let o = run(
        &d,
        &["play", IMG, "--mappings", "m.json", "--preset", "default"],
    );
    assert_eq!(o.rc, Some(2), "play, reversed order: {}", o.stderr);

    let o = run(&d, &["render", IMG, "--wav", "x.wav", "--preset", "nope"]);
    assert_eq!(o.rc, Some(2));
    assert!(
        o.stderr
            .contains("invalid value 'nope' for '--preset <NAME>'"),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains("[possible values: default]"),
        "{}",
        o.stderr
    );

    let o = run(
        &d,
        &[
            "render",
            "nope.png",
            "--wav",
            "x.wav",
            "--loop",
            "--gain",
            "0.5",
            "--mappings",
            "missing.json",
        ],
    );
    assert_eq!(o.rc, Some(1));
    assert!(
        o.stderr.contains("--loop requires unity --gain"),
        "loop refusal first: {}",
        o.stderr
    );
    assert!(
        !o.stderr.contains("missing.json"),
        "mappings not read yet: {}",
        o.stderr
    );
}

// ── L-30 precedence, library level, with a preset that actually sets something ──────────────

/// L-30: explicit flag > preset > default, via `value_source`. Slice 1's only preset sets nothing,
/// so the precedence is forced here with a SECOND preset parsed through the real manifest loader
/// (`parse_manifest`) and turned into a request exactly as `main.rs` does
/// (`preset_render_args` → `RenderRequest::from` → `overlay_preset`). For EVERY presettable id:
/// untyped ⇒ the preset's value; typed (even equal to the default) ⇒ the typed value; and the
/// options a preset may not set (`seed`, `loop`, the event dump, `ms-per-step`, `jitter-percent`)
/// never come from the preset even when its request differs there.
#[test]
fn l30_explicit_beats_preset_beats_default_for_every_presettable_id() {
    use audiohax::presets::{
        parse_manifest, parse_with_explicit, preset_render_args, PRESET_SETTING_IDS,
    };
    use audiohax::render::RenderRequest;
    // Preset values that differ from every default.
    let settings: &[(&str, &str)] = &[
        ("instruments", "6"),
        ("thickness", "0.3"),
        ("steps", "10"),
        ("expression", "on"),
        ("v2", "e3"),
        ("reverb", "off"),
        ("gain", "0.8"),
    ];
    for id in PRESET_SETTING_IDS {
        assert!(
            settings.iter().any(|(k, _)| k == id),
            "no probe for presettable `{id}`"
        );
    }
    let mut m: Value = serde_json::from_str(audiohax::presets::MANIFEST_JSON).expect("json");
    let mut p = m["presets"][0].clone();
    p["id"] = json!("probe");
    p["settings"] = Value::Object(
        settings
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v)))
            .collect(),
    );
    m["presets"].as_array_mut().expect("arr").push(p);
    let manifest = parse_manifest(&m.to_string()).expect("probe preset loads");
    let probe = manifest
        .presets
        .iter()
        .find(|p| p.id == "probe")
        .expect("probe");
    let preset_req = RenderRequest::from(&preset_render_args(probe).expect("args"));

    let req_of = |argv: &[&str]| {
        let (cli, ex) = parse_with_explicit(argv.to_vec()).expect("parse");
        match cli.command {
            audiohax::cli::Command::Render(r) => (RenderRequest::from(&r), ex),
            other => panic!("{other:?}"),
        }
    };
    let (plain, none) = req_of(&["audiohax", "render", "x.png"]);
    let laid = plain.overlay_preset(&preset_req, &none);
    assert_eq!(laid.pipeline.instruments, 6);
    assert_eq!(laid.pipeline.thickness, 0.3);
    assert_eq!(laid.pipeline.steps, 10);
    assert!(laid.expression.is_on());
    assert!(laid.v2.is_some());
    assert!(!laid.reverb.is_on());
    assert_eq!(laid.gain, 0.8);

    let defaults = [
        ("instruments", "4"),
        ("thickness", "0.1"),
        ("steps", "40"),
        ("expression", "off"),
        ("reverb", "on"),
        ("gain", "1"),
    ];
    for (id, dflt) in defaults {
        let flag = format!("--{id}");
        let (typed, ex) = req_of(&["audiohax", "render", "x.png", &flag, dflt]);
        assert!(ex.contains(id));
        let out = typed.overlay_preset(&preset_req, &ex);
        assert_eq!(
            out,
            {
                let mut want = laid.clone();
                match id {
                    "instruments" => want.pipeline.instruments = 4,
                    "thickness" => want.pipeline.thickness = 0.1,
                    "steps" => want.pipeline.steps = 40,
                    "expression" => want.expression = typed.expression,
                    "reverb" => want.reverb = typed.reverb,
                    "gain" => want.gain = 1.0,
                    _ => unreachable!(),
                }
                want
            },
            "`{id}` typed AT ITS DEFAULT must beat the preset and leave every other field to it"
        );
    }

    // Options a preset may not set never leak from the preset's request.
    let mut leaky = preset_req.clone();
    leaky.seed = Some(99);
    leaky.loop_cue = true;
    leaky.pipeline.ms_per_step = 9;
    leaky.pipeline.jitter_percent = 1.0;
    let (base, ex) = req_of(&[
        "audiohax",
        "render",
        "x.png",
        "--wav",
        "o.wav",
        "--event-dump",
        "d.json",
    ]);
    let out = base.overlay_preset(&leaky, &ex);
    assert_eq!(out.seed, base.seed);
    assert_eq!(out.loop_cue, base.loop_cue);
    assert_eq!(out.event_dump, base.event_dump);
    assert_eq!(out.pipeline.ms_per_step, base.pipeline.ms_per_step);
    assert_eq!(out.pipeline.jitter_percent, base.pipeline.jitter_percent);
}

// ── what the loader admits must compose without panicking ──────────────────────────────────

/// `prepare_play` (compose + timeline, no synthesis, no audio device) of `table` on every tracked
/// picture, in THIS (debug) build so debug assertions are live. Returns per-picture outcome.
fn compose_all(
    table: &audiohax::mapping_loader::MappingTable,
) -> Vec<(String, Result<u64, String>)> {
    use audiohax::render::{prepare_play, ImageInput, RenderRequest};
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images");
    let mut pics: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("images")
        .map(|e| e.expect("entry").path())
        .collect();
    pics.sort();
    let (cli, _) = audiohax::presets::parse_with_explicit([
        "audiohax", "render", "--seed", "7", "--steps", "6",
    ])
    .expect("parse");
    let audiohax::cli::Command::Render(args) = cli.command else {
        panic!("render")
    };
    let req = RenderRequest::from(&args);
    pics.into_iter()
        .map(|pic| {
            let name = pic.file_name().unwrap().to_string_lossy().into_owned();
            let t = audiohax::mapping_loader::rebuild_mapping_table(table);
            let req = req.clone();
            let r = std::panic::catch_unwind(move || {
                prepare_play(&req, ImageInput::Path(&pic), &t, &mut |_| Ok(()))
                    .map(|plan| plan.timeline.iter().map(|e| e.at_ms).max().unwrap_or(0))
                    .map_err(|e| e.to_string())
            });
            let r = match r {
                Ok(x) => x,
                Err(p) => Err(format!(
                    "PANIC: {}",
                    p.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default()
                )),
            };
            (name, r)
        })
        .collect()
}

/// The shipped table composes on every tracked picture (the control for the two cases below:
/// `compose_all` itself works).
#[test]
fn the_shipped_table_composes_on_every_tracked_picture() {
    let t = audiohax::mapping_loader::default_mappings().expect("embedded");
    let res = compose_all(&t);
    assert!(res.len() >= 6, "{res:?}");
    for (pic, r) in &res {
        let last = r.as_ref().unwrap_or_else(|e| panic!("{pic}: {e}"));
        assert!(
            *last > 0 && *last <= audiohax::render::MAX_PIECE_MS,
            "{pic}: {last} ms"
        );
    }
}

/// L-32 admits a form with ZERO sections and an EMPTY form catalogue (`≤ 32`; the security review's MB-6
/// asked `1..=32`; `s124-02` headline 3 measured "renders rc=0 on lena, one picture"). A sweep of
/// every tracked picture in a DEBUG build: whatever the loader admits must compose or be refused
/// as an error — never panic.
#[test]
fn zero_section_forms_admitted_by_the_loader_never_panic_in_compose() {
    use audiohax::mapping_loader::{load_checked, MappingsOrigin};
    let mut zero_every = shipped();
    for f in zero_every["composition"]["form_catalogue"]
        .as_array_mut()
        .expect("forms")
    {
        f["sections"] = json!([]);
    }
    let mut empty_cat = shipped();
    empty_cat["composition"]["form_catalogue"] = json!([]);
    for (name, v) in [
        ("every form 0 sections", zero_every),
        ("empty form_catalogue", empty_cat),
    ] {
        let t = load_checked(&v.to_string(), MappingsOrigin::Builtin)
            .unwrap_or_else(|e| panic!("{name}: the loader admits it today: {e}"));
        for (pic, r) in compose_all(&t) {
            if let Err(e) = &r {
                assert!(!e.starts_with("PANIC"), "{name} on {pic}: {e}");
            }
        }
    }
}

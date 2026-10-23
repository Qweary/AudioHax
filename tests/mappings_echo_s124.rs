//! What a mappings file can put into an error message.
//!
//! 1. Text the file supplies is shown ESCAPED. serde's "unknown variant `…`" message quotes the
//!    file's own string; a string carrying ESC, CSI or BEL must not reach the console raw, through
//!    the library's `Display`, through any error `source()` chain, or on the stderr of `render`
//!    and `audiohax-tui` (a cwd-planted `./assets/mappings.json` needs no flag at all).
//! 2. The 10-minute piece refusal prints as a plain sentence (`Error: This piece would last …`),
//!    not as a quoted Debug string, so a file name is escaped once, not twice.
//!
//! Every binary run happens in its own private directory under `CARGO_TARGET_TMPDIR`.

use audiohax::mapping_loader::{
    load_checked, load_mappings_file, MappingsOrigin, DEFAULT_MAPPINGS_JSON,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_audiohax");
const TUI: &str = env!("CARGO_BIN_EXE_audiohax-tui");
const IMG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/images/AudioHaxImg1.jpg"
);

/// ESC `[2J` (clear screen), ESC `[31m` (red), BEL, and the 8-bit CSI (U+009B) — the three
/// terminal-control characters the review named.
const HOSTILE: &str = "x\u{1b}[2J\u{1b}[31mred\u{7}bell\u{9b}31mcsi";

fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-echo-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

fn shipped() -> Value {
    serde_json::from_str(DEFAULT_MAPPINGS_JSON).expect("json")
}

/// The shipped table with the first bass pattern's `kind` (a closed enum) set to `kind`.
fn with_bass_kind(kind: &str) -> String {
    let mut v = shipped();
    v["composition"]["bass_pattern_catalogue"][0]["kind"] = json!(kind);
    serde_json::to_string(&v).expect("ser")
}

/// The shipped table with every tempo at `bpm` (1 BPM passes the floor and trips the 600 s cap).
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

fn plant(cwd: &Path, body: &str) {
    std::fs::create_dir_all(cwd.join("assets")).expect("assets dir");
    std::fs::write(cwd.join("assets/mappings.json"), body).expect("plant");
}

/// No terminal-control character survives (a trailing newline from the process is allowed).
fn assert_no_raw_controls(what: &str, msg: &str) {
    let raw: Vec<char> = msg
        .chars()
        .filter(|c| c.is_control() && *c != '\n')
        .collect();
    assert!(
        raw.is_empty(),
        "{what}: raw control characters {raw:?} reached the message: {msg:?}"
    );
}

/// The three hostile characters are shown, escaped.
fn assert_shown_escaped(what: &str, msg: &str) {
    for shown in ["\\u{1b}[2J", "\\u{1b}[31m", "\\u{7}", "\\u{9b}"] {
        assert!(
            msg.contains(shown),
            "{what}: {shown} not shown escaped: {msg}"
        );
    }
}

struct Out {
    rc: Option<i32>,
    stderr: String,
}

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
    while child.try_wait().expect("wait").is_none() {
        if t0.elapsed() > Duration::from_secs(secs) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{exe} {args:?}: still running after {secs} s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let o = child.wait_with_output().expect("output");
    Out {
        rc: o.status.code(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Library level: the refusal names the file and the problem, shows the hostile string escaped,
/// and neither its `Display`, its `Debug` nor any error in its `source()` chain carries a raw
/// control character.
#[test]
fn an_unknown_variant_with_terminal_controls_is_shown_escaped() {
    let e = load_checked(
        &with_bass_kind(HOSTILE),
        MappingsOrigin::File(PathBuf::from("m.json")),
    )
    .expect_err("an unknown variant is refused");
    let msg = e.to_string();
    assert!(
        msg.starts_with("The mappings file `m.json` is not a valid mappings file: unknown variant"),
        "{msg}"
    );
    assert!(msg.ends_with("Nothing was rendered."), "{msg}");
    assert_no_raw_controls("Display", &msg);
    assert_shown_escaped("Display", &msg);
    assert_no_raw_controls("Debug", &format!("{e:?}"));
    let mut src = std::error::Error::source(&e);
    while let Some(s) = src {
        assert_no_raw_controls("source chain", &s.to_string());
        src = s.source();
    }
}

/// CONTROL: ordinary text in the same message is untouched — backticks, quotes, a backslash and
/// a non-ASCII letter read exactly as the file wrote them; only control characters are escaped.
#[test]
fn ordinary_text_in_the_same_message_is_not_mangled() {
    let e = load_checked(
        &with_bass_kind("plain `q\"u\\ote` é"),
        MappingsOrigin::Builtin,
    )
    .expect_err("refused");
    let msg = e.to_string();
    assert!(
        msg.contains("unknown variant `plain `q\"u\\ote` é`"),
        "ordinary text kept as written: {msg}"
    );
    assert!(msg.contains("expected one of"), "{msg}");
}

/// A raw 0x9B byte (not valid UTF-8) is refused as "not UTF-8 text" and nothing of it is echoed.
#[test]
fn a_raw_csi_byte_is_refused_as_not_utf8_and_not_echoed() {
    let d = scratch("raw9b");
    let mut body = with_bass_kind("PLACEHOLDER").into_bytes();
    let at = body
        .windows(11)
        .position(|w| w == b"PLACEHOLDER")
        .expect("placeholder");
    body.splice(
        at..at + 11,
        [0x1b, b'[', b'2', b'J', 0x9b, b'3', b'1', b'm'],
    );
    let p = d.join("raw.json");
    std::fs::write(&p, &body).expect("write");
    let msg = load_mappings_file(&p).expect_err("refused").to_string();
    assert!(msg.contains("is not UTF-8 text"), "{msg}");
    assert_no_raw_controls("not-UTF-8", &msg);
}

/// At the binaries, the way the review measured it: a cwd-planted `./assets/mappings.json` (no
/// flag) on `render` and on `audiohax-tui`, and the same file through `--mappings`. Each is
/// refused, names the file, and puts no raw control character on stderr.
#[test]
fn no_raw_control_reaches_stderr_from_render_or_the_tui() {
    let cwd = scratch("plant");
    plant(&cwd, &with_bass_kind(HOSTILE));
    let o = run_in(BIN, &cwd, &["render", IMG, "--wav", "x.wav"], 240);
    assert_eq!(o.rc, Some(1), "render refused: {}", o.stderr);
    assert!(
        o.stderr
            .contains("`assets/mappings.json` is not a valid mappings file"),
        "{}",
        o.stderr
    );
    assert_no_raw_controls("render (planted)", &o.stderr);
    assert_shown_escaped("render (planted)", &o.stderr);
    assert!(!cwd.join("x.wav").exists(), "nothing was rendered");

    let o = run_in(TUI, &cwd, &[], 60);
    assert_ne!(o.rc, Some(0), "tui refused: {}", o.stderr);
    assert!(
        o.stderr
            .contains("`assets/mappings.json` is not a valid mappings file"),
        "{}",
        o.stderr
    );
    assert_no_raw_controls("audiohax-tui (planted)", &o.stderr);

    let d = scratch("flag");
    std::fs::write(d.join("evil.json"), with_bass_kind(HOSTILE)).expect("write");
    let o = run_in(
        BIN,
        &d,
        &["render", IMG, "--wav", "x.wav", "--mappings", "evil.json"],
        240,
    );
    assert_eq!(o.rc, Some(1), "{}", o.stderr);
    assert!(o.stderr.contains("`evil.json`"), "{}", o.stderr);
    assert_no_raw_controls("render --mappings", &o.stderr);
}

/// The 10-minute refusal is a plain sentence on `render` and on `play`: no surrounding quotes,
/// and a file name holding a double quote is escaped ONCE (by the file-name escaping), not twice.
#[test]
fn the_ten_minute_refusal_prints_as_a_sentence_not_a_quoted_string() {
    let d = scratch("cap");
    let name = "slow\"1.json";
    std::fs::write(d.join(name), all_tempi(1.0)).expect("write");
    for args in [
        vec!["render", IMG, "--wav", "slow.wav", "--mappings", name],
        vec!["play", IMG, "--output", "midi", "--mappings", name],
    ] {
        let o = run_in(BIN, &d, &args, 240);
        assert_eq!(o.rc, Some(1), "{args:?}: {}", o.stderr);
        assert!(
            o.stderr.starts_with("Error: This piece would last "),
            "{args:?}: plain sentence, no opening quote: {:?}",
            o.stderr
        );
        assert!(
            o.stderr
                .contains("The mappings file `slow\\\"1.json` sets this piece's tempo"),
            "{args:?}: file name escaped once: {:?}",
            o.stderr
        );
        assert!(
            !o.stderr.contains("slow\\\\\\\"1.json"),
            "{args:?}: escaped twice: {:?}",
            o.stderr
        );
        assert!(
            o.stderr
                .trim_end()
                .ends_with("`composition.form_catalogue[].sections`)."),
            "{args:?}: no closing quote: {:?}",
            o.stderr
        );
    }
    assert!(!d.join("slow.wav").exists(), "no WAV");
}

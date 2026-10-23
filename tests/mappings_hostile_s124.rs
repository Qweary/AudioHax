//! S124 Test Engineer (Specialist 5): the security review's hostile mappings files (`s124-SH` § 5.3, H1–H15,
//! plus the S124 addendum's 1 BPM floor), each written to a FILE under a private scratch dir and
//! loaded through the library's one bounded, validated chokepoint
//! (`mapping_loader::load_mappings_file` = `read_mappings_file` → `load_checked`).
//!
//! Every hostile case must be REFUSED with a message that names the file AND the field (or, for
//! read-level refusals, the file and the reason), says nothing was rendered, and carries no Rust
//! `Debug` text. None may panic, abort or hang: the special-file cases (FIFO, `/dev/zero`, a 1 GiB
//! sparse file) run behind a wall-clock guard.
//!
//! Pure (no image, no audio): `mapping_loader` builds under every feature set, so this file runs
//! under `cargo test`, `cargo test --features gui` AND `cargo test --no-default-features`.
//!
//! Scratch files live under cargo's `CARGO_TARGET_TMPDIR` (inside the target dir), never `/tmp`.

use audiohax::mapping_loader::{
    default_mappings, load_checked, load_mappings_file, read_mappings_file, MappingsError,
    MappingsOrigin, DEFAULT_MAPPINGS_JSON, MAX_BPM, MAX_MAPPINGS_BYTES, MAX_SECTIONS_PER_FORM,
    MIN_BPM,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── scaffolding ─────────────────────────────────────────────────────────────────────────────

/// A fresh, private scratch directory for one test.
fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-hostile-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

/// The shipped table as JSON.
fn shipped() -> Value {
    serde_json::from_str(DEFAULT_MAPPINGS_JSON).expect("shipped mappings are JSON")
}

/// Write `v` (compact) to `dir/name` and return the path.
fn write_json(dir: &Path, name: &str, v: &Value) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, serde_json::to_string(v).expect("ser")).expect("write");
    p
}

/// Run `f` on another thread; fail (not hang) if it takes longer than `secs`.
fn within<T: Send + 'static>(secs: u64, what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(secs))
        .unwrap_or_else(|_| panic!("{what}: did not return within {secs} s (blocked or hung)"))
}

/// The user-facing sentence for a load of `path`, which MUST be a refusal.
fn refusal(path: &Path) -> String {
    let p = path.to_path_buf();
    let got = within(20, &format!("load {}", path.display()), move || {
        load_mappings_file(&p).map(|_| ())
    });
    match got {
        Ok(()) => panic!("{}: a hostile file was ACCEPTED", path.display()),
        Err(e) => {
            let msg = e.to_string();
            // `Debug` is the sentence too (main.rs prints `Error: {:?}`), MB-8.
            assert_eq!(format!("{e:?}"), msg, "Debug must equal Display");
            assert_no_debug_text(&msg);
            assert!(
                msg.ends_with("Nothing was rendered."),
                "MB-8: every refusal says nothing was rendered: {msg}"
            );
            msg
        }
    }
}

fn assert_no_debug_text(msg: &str) {
    for bad in ["Os {", "Kind(", "kind:", "Error {"] {
        assert!(!msg.contains(bad), "Rust Debug text `{bad}` in: {msg}");
    }
}

/// Assert the refusal names the file (as the user typed it) and the field.
fn assert_names(msg: &str, path: &Path, field: &str) {
    let shown = path.display().to_string();
    assert!(
        msg.contains(&format!("`{shown}`")),
        "the message must name the file `{shown}`: {msg}"
    );
    assert!(
        msg.contains(field),
        "the message must name the field `{field}`: {msg}"
    );
}

/// Sets one field of a table to a value.
type Setter = Box<dyn Fn(&mut Value, Value)>;

const CHARACTERS: [&str; 10] = [
    "drone", "lament", "nocturne", "hymn", "ballad", "waltz", "lilt", "march", "gigue", "scherzo",
];

// ── H10 / MB-2: the embedded table passes the chokepoint ────────────────────────────────────

/// MB-2 / H10: the embedded default passes validation through the one chokepoint, and the same
/// bytes written to a file pass the bounded FILE path too (the shipped file is a regular, small,
/// UTF-8 file: if the file path refused it, every cwd render would break).
#[test]
fn h10_the_shipped_table_passes_through_both_entries() {
    default_mappings().expect("embedded default passes");
    let d = scratch("h10");
    let p = d.join("mappings.json");
    std::fs::write(&p, DEFAULT_MAPPINGS_JSON).expect("write");
    let t = load_mappings_file(&p).expect("shipped bytes from a file pass");
    let e = default_mappings().expect("embedded");
    assert_eq!(t.composition, e.composition, "same table from both entries");
    // And it is the repo file, byte for byte (the embedded copy is not a stale snapshot).
    let repo =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mappings.json"))
            .expect("repo file");
    assert_eq!(repo, DEFAULT_MAPPINGS_JSON);
}

// ── H1: inverted tempo windows, all ten characters ──────────────────────────────────────────

/// H1 (was a `f32::clamp` panic): `bpm_min > bpm_max` for EACH of the ten characters is refused,
/// naming `composition.affect.character_tempo.<name>`.
#[test]
fn h1_inverted_tempo_window_is_refused_for_each_character_from_a_file() {
    let d = scratch("h1");
    for ch in CHARACTERS {
        let mut v = shipped();
        v["composition"]["affect"]["character_tempo"][ch] = json!({"bpm_min": 200, "bpm_max": 50});
        let p = write_json(&d, &format!("inverted-{ch}.json"), &v);
        let msg = refusal(&p);
        assert_names(
            &msg,
            &p,
            &format!("composition.affect.character_tempo.{ch}"),
        );
        assert!(
            msg.contains("greater than bpm_max"),
            "{ch}: says what is wrong: {msg}"
        );
    }
}

// ── H2 / H4: non-finite numbers (serde turns 1e39 into inf as f32) ──────────────────────────

/// H2: `energy_tempo.bpm_at_zero: 1e39` (an f32 `inf`) is refused as not finite, naming the path.
/// H4: the same plus a 1,000-section form (was a 296 GB allocation abort) is refused BEFORE
/// compose, and the process is still alive to assert it.
#[test]
fn h2_h4_non_finite_numbers_are_refused_before_anything_is_composed() {
    let d = scratch("h2");
    let mut v = shipped();
    v["composition"]["affect"]["energy_tempo"]["bpm_at_zero"] = json!(1e39);
    let p = write_json(&d, "inf.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "composition.affect.energy_tempo.bpm_at_zero");
    assert!(msg.contains("not finite"), "{msg}");
    assert!(
        msg.contains("1e39"),
        "a person-readable number, not 40 digits: {msg}"
    );

    let section = v["composition"]["form_catalogue"][0]["sections"][0].clone();
    v["composition"]["form_catalogue"][0]["sections"] = Value::Array(vec![section; 1000]);
    let p = write_json(&d, "inf-1000-sections.json", &v);
    let t0 = Instant::now();
    let msg = refusal(&p);
    assert!(msg.contains("not finite"), "{msg}");
    assert!(
        t0.elapsed() < Duration::from_secs(5),
        "refused promptly (no compose): {:?}",
        t0.elapsed()
    );

    // A non-finite number in a key NO code reads is still refused (MB-4 walks every number).
    let mut v = shipped();
    v["composition"]["affect"]["zz_unread_key"] = json!([1, 2, 1e39]);
    let p = write_json(&d, "inf-in-unread-key.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "composition.affect.zz_unread_key[2]");
}

// ── H3: section counts ──────────────────────────────────────────────────────────────────────

/// H3: a form with 33 sections is refused naming the form and the field; 32 is accepted (the
/// bound is inclusive, L-32); 10,000 is refused (by the byte cap or the section cap, whichever
/// trips first — both name the file).
#[test]
fn h3_section_count_is_capped_at_32_and_named() {
    let d = scratch("h3");
    let base = shipped();
    let section = base["composition"]["form_catalogue"][2]["sections"][0].clone();
    let form_id = base["composition"]["form_catalogue"][2]["id"]
        .as_str()
        .expect("id")
        .to_string();

    let mut v = base.clone();
    v["composition"]["form_catalogue"][2]["sections"] =
        Value::Array(vec![section.clone(); MAX_SECTIONS_PER_FORM + 1]);
    let p = write_json(&d, "33.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "composition.form_catalogue[2].sections");
    assert!(
        msg.contains(&format!("`{form_id}`")),
        "names the form id: {msg}"
    );
    assert!(msg.contains("has 33 sections; the limit is 32"), "{msg}");

    let mut v = base.clone();
    v["composition"]["form_catalogue"][2]["sections"] =
        Value::Array(vec![section.clone(); MAX_SECTIONS_PER_FORM]);
    let p = write_json(&d, "32.json", &v);
    load_mappings_file(&p).expect("32 sections is inside the bound");

    let mut v = base;
    v["composition"]["form_catalogue"][2]["sections"] = Value::Array(vec![section; 10_000]);
    let p = write_json(&d, "10000.json", &v);
    let msg = refusal(&p);
    assert!(
        msg.contains("the limit is 1 MiB") || msg.contains("the limit is 32"),
        "{msg}"
    );
    assert!(msg.contains(&format!("`{}`", p.display())), "{msg}");
}

// ── H5 + the S124 addendum: the tempo window is [1, 400] BPM inclusive ──────────────────────

/// L-41 / addendum A.1: the floor is 1 BPM INCLUSIVE and the ceiling 400 inclusive, on every BPM
/// field. 1 and 400 load; 0.999, 0.5, 0, -1, 400.5 and 1e30 are refused naming the field and the
/// window. H5: 0.001 and 1e30 with `character_tempo: {}` are refused too.
#[test]
fn h5_tempo_floor_is_one_bpm_inclusive_and_ceiling_400() {
    assert_eq!(MIN_BPM, 1.0, "L-41: the floor is 1 BPM");
    assert_eq!(MAX_BPM, 400.0);
    let d = scratch("h5");
    let fields: [(&str, Setter); 4] = [
        (
            "composition.affect.energy_tempo.bpm_at_zero",
            Box::new(|v, x| v["composition"]["affect"]["energy_tempo"]["bpm_at_zero"] = x),
        ),
        (
            "composition.affect.energy_tempo.bpm_at_one",
            Box::new(|v, x| v["composition"]["affect"]["energy_tempo"]["bpm_at_one"] = x),
        ),
        (
            "composition.affect.character_tempo.drone.bpm_min",
            Box::new(|v, x| v["composition"]["affect"]["character_tempo"]["drone"]["bpm_min"] = x),
        ),
        (
            "composition.affect.character_tempo.scherzo.bpm_max",
            Box::new(|v, x| {
                v["composition"]["affect"]["character_tempo"]["scherzo"]["bpm_max"] = x
            }),
        ),
    ];
    for (field, set) in &fields {
        // Accepted at the two edges. (`bpm_max` of scherzo at 1 would invert its window, so the
        // floor edge is checked with the window collapsed to [1, 1].)
        for ok in [1.0, 400.0] {
            let mut v = shipped();
            set(&mut v, json!(ok));
            if field.ends_with("drone.bpm_min") && ok == 400.0 {
                v["composition"]["affect"]["character_tempo"]["drone"]["bpm_max"] = json!(400.0);
            }
            if field.ends_with("scherzo.bpm_max") && ok == 1.0 {
                v["composition"]["affect"]["character_tempo"]["scherzo"]["bpm_min"] = json!(1.0);
            }
            let p = write_json(&d, "edge.json", &v);
            load_mappings_file(&p)
                .unwrap_or_else(|e| panic!("{field} = {ok} must be accepted (L-41): {e}"));
        }
        for bad in [0.999, 0.5, 0.0, -1.0, 400.5, 1e30] {
            let mut v = shipped();
            set(&mut v, json!(bad));
            let p = write_json(&d, "bad.json", &v);
            let msg = refusal(&p);
            assert_names(&msg, &p, field);
            assert!(
                msg.contains("a tempo must be between 1 and 400 BPM"),
                "{field} = {bad}: {msg}"
            );
        }
    }
    // H5 proper: tiny / huge energy tempo with no character rails to clamp it.
    for bad in [0.001, 1e30] {
        let mut v = shipped();
        v["composition"]["affect"]["character_tempo"] = json!({});
        v["composition"]["affect"]["energy_tempo"]["bpm_at_zero"] = json!(bad);
        let p = write_json(&d, "norail.json", &v);
        let msg = refusal(&p);
        assert_names(&msg, &p, "composition.affect.energy_tempo.bpm_at_zero");
    }
    // Legacy integer table: 0 and u32::MAX refused, 1 accepted.
    for (bpm, ok) in [
        (0u64, false),
        (1, true),
        (400, true),
        (401, false),
        (4_294_967_295, false),
    ] {
        let mut v = shipped();
        v["global"]["brightness_to_tempo_bpm"]["0-30"] = json!(bpm);
        let p = write_json(&d, "legacy.json", &v);
        if ok {
            load_mappings_file(&p).unwrap_or_else(|e| panic!("legacy {bpm}: {e}"));
        } else {
            let msg = refusal(&p);
            assert_names(&msg, &p, "global.brightness_to_tempo_bpm.0-30");
        }
    }
}

// ── H6: MIDI program bytes ──────────────────────────────────────────────────────────────────

/// H6 (was 0xFF System Reset on the MIDI wire): programs above 127 in an ensemble and in a pool
/// entry are refused naming the exact index; 127 is accepted.
#[test]
fn h6_programs_above_127_are_refused_with_the_index() {
    let d = scratch("h6");
    let mut v = shipped();
    v["composition"]["ensemble_catalogue"][0]["programs"] = json!([255, 200, 128, 240]);
    let p = write_json(&d, "ens.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "composition.ensemble_catalogue[0].programs[0]");
    assert!(msg.contains("255") && msg.contains("0-127"), "{msg}");

    let mut v = shipped();
    v["composition"]["ensemble_catalogue"][3]["programs"][2] = json!(128);
    let p = write_json(&d, "ens128.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "composition.ensemble_catalogue[3].programs[2]");

    let mut v = shipped();
    v["composition"]["instrumentation_pools"]["roles"][0]["entries"][0]["program"] = json!(255);
    let p = write_json(&d, "pool.json", &v);
    let msg = refusal(&p);
    assert_names(
        &msg,
        &p,
        "composition.instrumentation_pools.roles[0].entries[0].program",
    );

    let mut v = shipped();
    v["composition"]["ensemble_catalogue"][0]["programs"][0] = json!(127);
    let p = write_json(&d, "ens127.json", &v);
    load_mappings_file(&p).expect("127 is a MIDI program");
}

// ── H8: what is not a small regular UTF-8 file ──────────────────────────────────────────────

/// H8: a directory, `/dev/zero` (and a symlink to it), a 1 MiB + 1 file, a 1 GiB SPARSE file,
/// non-UTF-8 bytes, a missing path and an empty file each get their MB-1 sentence, promptly, and
/// none reads the whole object (the sparse GiB is refused from metadata in well under a second).
/// A file of EXACTLY 1 MiB is accepted (the bound is inclusive).
#[test]
fn h8_read_level_refusals_are_prompt_and_name_the_file() {
    let d = scratch("h8");

    let msg = refusal(&d);
    assert!(msg.contains("is not a regular file"), "directory: {msg}");

    #[cfg(unix)]
    {
        let msg = refusal(Path::new("/dev/zero"));
        assert!(msg.contains("`/dev/zero` is not a regular file"), "{msg}");
        let link = d.join("zero-link.json");
        std::os::unix::fs::symlink("/dev/zero", &link).expect("symlink");
        let msg = refusal(&link);
        assert!(
            msg.contains("is not a regular file"),
            "symlink → /dev/zero: {msg}"
        );
        // A symlink to a GOOD file is followed (the user named it).
        let good = d.join("good.json");
        std::fs::write(&good, DEFAULT_MAPPINGS_JSON).expect("write");
        let glink = d.join("good-link.json");
        std::os::unix::fs::symlink(&good, &glink).expect("symlink");
        load_mappings_file(&glink).expect("a symlink to a valid file is followed");
    }

    let over = d.join("over.json");
    std::fs::write(&over, vec![b' '; MAX_MAPPINGS_BYTES as usize + 1]).expect("write");
    let msg = refusal(&over);
    assert_names(
        &msg,
        &over,
        "is 1048577 bytes; the limit is 1 MiB (1048576 bytes)",
    );

    let sparse = d.join("sparse.json");
    std::fs::File::create(&sparse)
        .and_then(|f| f.set_len(1 << 30))
        .expect("sparse");
    let t0 = Instant::now();
    let msg = refusal(&sparse);
    assert!(msg.contains("is 1073741824 bytes"), "{msg}");
    assert!(
        t0.elapsed() < Duration::from_secs(1),
        "refused from metadata, not by reading a GiB: {:?}",
        t0.elapsed()
    );

    // Exactly 1 MiB: the shipped JSON padded with trailing whitespace to the byte.
    let exact = d.join("exact.json");
    let mut bytes = DEFAULT_MAPPINGS_JSON.as_bytes().to_vec();
    bytes.resize(MAX_MAPPINGS_BYTES as usize, b'\n');
    std::fs::write(&exact, &bytes).expect("write");
    load_mappings_file(&exact).expect("a file of exactly 1 MiB is inside the bound");

    let bin = d.join("latin1.json");
    std::fs::write(&bin, [b'{', 0xff, 0xfe, b'}']).expect("write");
    let msg = refusal(&bin);
    assert_names(&msg, &bin, "is not UTF-8 text");

    let missing = d.join("nope.json");
    let msg = refusal(&missing);
    assert_names(&msg, &missing, "could not be read");

    let empty = d.join("empty.json");
    std::fs::write(&empty, b"").expect("write");
    let msg = refusal(&empty);
    assert_names(&msg, &empty, "is not a valid mappings file");
    assert!(msg.contains("line 1 column 0"), "{msg}");
}

/// H8 FIFO: a named pipe given as the mappings file is refused from the PATH's metadata before
/// `open()` (which would block forever waiting for a writer). Guarded: if the loader blocks, the
/// test FAILS after 10 s instead of hanging, and unblocks the reader on the way out.
#[cfg(unix)]
#[test]
fn h8_a_fifo_is_refused_without_blocking() {
    let d = scratch("fifo");
    let fifo = d.join("pipe.json");
    let st = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(st.success(), "mkfifo");
    let p = fifo.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(
            load_mappings_file(&p)
                .map(|_| ())
                .map_err(|e| e.to_string()),
        );
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Err(msg)) => {
            assert_names(&msg, &fifo, "is not a regular file");
            assert_no_debug_text(&msg);
        }
        Ok(Ok(())) => panic!("a FIFO was accepted as a mappings file"),
        Err(_) => {
            // Unblock the stuck reader so the test binary can exit, then fail.
            let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
            panic!("the loader BLOCKED on a FIFO for 10 s (MB-1 says refuse before open)");
        }
    }
}

/// MB-1 "unreadable" (a present file the user cannot read) is reported, not replaced. Skipped
/// when the test runs with privileges that ignore file modes (root).
#[cfg(unix)]
#[test]
fn h8_an_unreadable_file_is_reported_without_debug_text() {
    use std::os::unix::fs::PermissionsExt;
    let d = scratch("perm");
    let p = d.join("locked.json");
    std::fs::write(&p, DEFAULT_MAPPINGS_JSON).expect("write");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::read(&p).is_ok() {
        eprintln!("SKIPPED: running with privileges that ignore mode 000; nothing was checked");
        return;
    }
    let msg = refusal(&p);
    assert_names(&msg, &p, "could not be read: Permission denied");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).expect("chmod back");
}

// ── H9: syntax-level hostility ──────────────────────────────────────────────────────────────

/// H9: a NaN literal, `1e400`, a UTF-8 BOM, trailing garbage and a duplicate top-level key are each
/// a parse refusal that names the file and gives serde's line and column. (BOM acceptance, MB-15,
/// is NICE and not built: the BOM case is pinned as a refusal so a later change to accept it is a
/// visible decision.)
#[test]
fn h9_syntax_hostility_is_a_parse_refusal_with_line_and_column() {
    let d = scratch("h9");
    let text = DEFAULT_MAPPINGS_JSON;
    assert!(
        text.contains("\"bpm_at_zero\": 52.0"),
        "anchor for the edits"
    );
    let cases: Vec<(&str, String)> = vec![
        (
            "nan",
            text.replacen("\"bpm_at_zero\": 52.0", "\"bpm_at_zero\": NaN", 1),
        ),
        (
            "e400",
            text.replacen("\"bpm_at_zero\": 52.0", "\"bpm_at_zero\": 1e400", 1),
        ),
        ("bom", format!("\u{feff}{text}")),
        ("trailing", format!("{text}\n}}garbage")),
        ("dup", text.replacen('{', "{\"global\": {},", 1)),
    ];
    for (name, body) in cases {
        let p = d.join(format!("{name}.json"));
        std::fs::write(&p, body).expect("write");
        let msg = refusal(&p);
        assert_names(&msg, &p, "is not a valid mappings file");
        assert!(
            msg.contains(" line ") && msg.contains(" column "),
            "{name}: serde's position: {msg}"
        );
    }
}

// ── H14: the console is not the file's to paint ────────────────────────────────────────────

/// H14 / MB-8: a file NAME carrying a terminal escape, and a form ID carrying one, are shown
/// escaped; the raw ESC byte never reaches the message.
#[cfg(unix)]
#[test]
fn h14_escapes_in_the_file_name_and_in_an_id_are_neutralised() {
    let d = scratch("h14");
    let mut v = shipped();
    let section = v["composition"]["form_catalogue"][0]["sections"][0].clone();
    v["composition"]["form_catalogue"][0]["id"] = json!("evil\u{1b}[31mred");
    v["composition"]["form_catalogue"][0]["sections"] = Value::Array(vec![section; 40]);
    let p = write_json(&d, "name\u{1b}[2Jclear.json", &v);
    let msg = refusal(&p);
    assert!(
        !msg.contains('\u{1b}'),
        "raw ESC reached the console: {msg:?}"
    );
    assert!(msg.contains("\\u{1b}"), "shown escaped: {msg}");
    assert!(
        msg.contains("composition.form_catalogue[0].sections"),
        "{msg}"
    );
}

// ── H15: regression guards for shapes measured safe ─────────────────────────────────────────

/// H15 (NICE): a 100,000-deep array in an ignored key cannot overflow the stack (serde's recursion
/// limit refuses it as a parse error, with the file named).
#[test]
fn h15_deep_nesting_is_refused_not_a_stack_overflow() {
    let d = scratch("h15");
    let text = DEFAULT_MAPPINGS_JSON.replacen(
        '{',
        &format!(
            "{{\"_deep\": {}0{},",
            "[".repeat(100_000),
            "]".repeat(100_000)
        ),
        1,
    );
    let p = d.join("deep.json");
    std::fs::write(&p, text).expect("write");
    let msg = within(20, "deep", {
        let p = p.clone();
        move || {
            load_mappings_file(&p)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    })
    .expect_err("a 100,000-deep array is not a mappings file");
    assert!(msg.contains(&format!("`{}`", p.display())), "{msg}");
}

/// L-32: the unknown-key policy stays PERMISSIVE in slice 1 (the security review HL-9: the shipped file
/// carries keys no code reads). Pinned so a change to refuse them is a visible decision.
#[test]
fn unknown_keys_are_still_accepted_in_slice_1() {
    let d = scratch("unknown");
    let mut v = shipped();
    v["zz_not_a_key"] = json!({"anything": [1, 2, 3]});
    v["composition"]["affect"]["zz_also_not"] = json!(7);
    let p = write_json(&d, "unknown.json", &v);
    load_mappings_file(&p).expect("unknown keys are allowed (L-32)");
}

// ── the chokepoint, not only the file reader ────────────────────────────────────────────────

/// MB-2: `load_checked` (the chokepoint the embedded default and presets use) applies the same
/// bounds as the file path, and names its origin — the built-in table is called "The built-in
/// mappings", a file is named by path.
#[test]
fn load_checked_applies_the_bounds_and_names_the_origin() {
    let mut v = shipped();
    v["composition"]["affect"]["character_tempo"]["march"] = json!({"bpm_min": 200, "bpm_max": 50});
    let text = serde_json::to_string(&v).expect("ser");
    let e = load_checked(&text, MappingsOrigin::Builtin).expect_err("refused");
    let msg = e.to_string();
    assert!(
        msg.starts_with("The built-in mappings: `composition.affect.character_tempo.march`"),
        "{msg}"
    );
    let e =
        load_checked(&text, MappingsOrigin::File(PathBuf::from("x/y.json"))).expect_err("refused");
    assert!(
        e.to_string().starts_with("The mappings file `x/y.json`:"),
        "{e}"
    );
    assert!(matches!(e, MappingsError::Invalid { .. }));
    // `read_mappings_file` alone bounds but does not validate: the same text READS fine.
    let d = scratch("chk");
    let p = d.join("inv.json");
    std::fs::write(&p, &text).expect("write");
    read_mappings_file(&p).expect("reading is not validating");
    refusal(&p);
}

/// A table without a `composition` block (the legacy schema) still passes when its integer
/// tempi are in range, and is still checked when they are not.
#[test]
fn a_legacy_table_without_composition_is_bounded_too() {
    let d = scratch("legacy");
    let mut v = shipped();
    v.as_object_mut().expect("obj").remove("composition");
    let p = write_json(&d, "legacy-ok.json", &v);
    let t = load_mappings_file(&p).expect("legacy table passes");
    assert!(t.composition.is_none());
    v["global"]["brightness_to_tempo_bpm"]["71-100"] = json!(0);
    let p = write_json(&d, "legacy-bad.json", &v);
    let msg = refusal(&p);
    assert_names(&msg, &p, "global.brightness_to_tempo_bpm.71-100");
}

/// L-32 says "≤ 32 sections per form"; the security review's MB-6 says `1..=32`. The build follows L-32: a
/// form with ZERO sections and an EMPTY form catalogue both pass the loader. Pinned here as the
/// current behaviour (the lead was asked to confirm, `s124-02` headline 3); whether they compose
/// safely is tested in `preset_cli_s124.rs` (needs the image + synth features).
#[test]
fn zero_section_form_and_empty_catalogue_pass_the_loader_today() {
    let d = scratch("zero");
    let mut v = shipped();
    v["composition"]["form_catalogue"][0]["sections"] = json!([]);
    let p = write_json(&d, "zero.json", &v);
    load_mappings_file(&p).expect("L-32's text: 0 ≤ 32");
    let mut v = shipped();
    v["composition"]["form_catalogue"] = json!([]);
    let p = write_json(&d, "empty-cat.json", &v);
    load_mappings_file(&p).expect("an empty catalogue passes the loader");
}

/// L-32: "ONE validation function in the library, run on EVERY mappings load". `audiohax::tui::
/// build_engine(path, …)` is a PUBLIC library function that still loads a mappings file through
/// the legacy unchecked `load_mappings` (no size bound, no validation). No front-end calls it in
/// slice 1 (the `audiohax-tui` bin re-implements it over the checked loader because `src/tui.rs`
/// was outside the implementer's write set, `s124-02` headline 9), so this is a library-API
/// bypass, not a reachable CLI hole.
#[test]
fn f1_tui_build_engine_goes_through_the_chokepoint() {
    let d = scratch("f1");
    let mut v = shipped();
    v["composition"]["affect"]["character_tempo"]["march"] = json!({"bpm_min": 200, "bpm_max": 50});
    let p = write_json(&d, "inverted.json", &v);
    let got = audiohax::tui::build_engine(p.to_str().expect("utf8"), 5, 3);
    assert!(
        got.is_err(),
        "an inverted tempo window (refused by the chokepoint) was accepted by tui::build_engine"
    );
}

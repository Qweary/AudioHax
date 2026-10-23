//! S124 Test Engineer (Specialist 5): the preset MANIFEST and the two new CLI flags, through the
//! public library API only (`audiohax::presets`, `audiohax::cli`, `audiohax::options_schema`).
//!
//! Covers: L-28 "a field slice 1 cannot honour is REFUSED at manifest load, never silently
//! ignored" (on the default AND on a second preset, at either position); `deny_unknown_fields` at
//! every level; PB-1 (every non-presettable schema id refused, by enumeration of the schema, not of
//! a hand list); PB-2 (values outside the window's bounds refused); L-30 mutual exclusion of
//! `--preset` / `--mappings` on `play` and `render` in both orders, and the typed-set read from
//! clap's `value_source` (typed-equal-to-default counts); L-30's "only these two flags reach
//! `--help`", and only under `play` / `render`.
//!
//! Pure (clap + serde): runs under every feature set, `--no-default-features` included.

use audiohax::cli::{Cli, Command};
use audiohax::options_schema::{render_options_schema, Tier};
use audiohax::presets::{
    default_preset, find, manifest, parse_manifest, parse_with_explicit, preset_ids,
    preset_render_args, validate_against_cli, Manifest, PresetError, MANIFEST_JSON,
    PRESET_SETTING_IDS,
};
use clap::CommandFactory;
use serde_json::{json, Value};

/// One JSON edit applied to the shipped manifest.
type Edit = Box<dyn Fn(&mut Value)>;

fn shipped() -> Value {
    serde_json::from_str(MANIFEST_JSON).expect("manifest is JSON")
}

/// The shipped manifest with a second preset (a copy of `default`, id `second`) edited by `edit`,
/// placed FIRST or LAST in the list.
fn with_second(first: bool, edit: impl FnOnce(&mut Value)) -> Result<Manifest, PresetError> {
    let mut v = shipped();
    let mut p = v["presets"][0].clone();
    p["id"] = json!("second");
    p["name"] = json!("Second");
    edit(&mut p);
    let list = v["presets"].as_array_mut().expect("array");
    if first {
        list.insert(0, p);
    } else {
        list.push(p);
    }
    parse_manifest(&v.to_string())
}

fn with_default(edit: impl FnOnce(&mut Value)) -> Result<Manifest, PresetError> {
    let mut v = shipped();
    edit(&mut v["presets"][0]);
    parse_manifest(&v.to_string())
}

// ── the embedded manifest ───────────────────────────────────────────────────────────────────

/// L-28 / PB-4: the embedded manifest is the file on disk, parses, validates, holds exactly the
/// one `default` preset, and that preset's clap arguments are clap's all-defaults render.
#[test]
fn embedded_manifest_is_the_file_and_holds_only_default() {
    let disk = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/presets/manifest.json"
    ))
    .expect("manifest file");
    assert_eq!(
        disk, MANIFEST_JSON,
        "the embedded manifest is the repo file"
    );
    let m = manifest().expect("embedded manifest validates");
    assert_eq!(preset_ids(), vec!["default"]);
    assert_eq!(m.default, "default");
    let d = default_preset().expect("default");
    assert!(std::ptr::eq(d, find("default").expect("find")));
    let (cli, _) = parse_with_explicit(["audiohax", "render"]).expect("parse");
    let Command::Render(defaults) = cli.command else {
        panic!("render")
    };
    assert_eq!(preset_render_args(d).expect("args"), defaults);
    for p in &m.presets {
        validate_against_cli(p).unwrap_or_else(|e| panic!("embedded preset {}: {e}", p.id));
    }
    assert!(matches!(find("nope"), Err(PresetError::Unknown(_))));
}

/// PB-3 / L-32: the manifest names no file path and no output option. Swept over the raw JSON
/// strings, so a path smuggled into ANY field (description, provenance) is caught too.
#[test]
fn embedded_manifest_names_no_path_and_no_output_option() {
    fn strings(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(o) => o.iter().for_each(|(k, x)| {
                out.push(k.clone());
                strings(x, out)
            }),
            _ => {}
        }
    }
    let mut all = Vec::new();
    strings(&shipped(), &mut all);
    for s in &all {
        for bad in [
            ".sf2",
            ".wav",
            ".json",
            "--wav",
            "--event-dump",
            "--soundfont",
            "/",
        ] {
            assert!(!s.contains(bad), "manifest string {s:?} contains {bad:?}");
        }
    }
}

// ── L-28: refuse, never ignore ──────────────────────────────────────────────────────────────

/// Each field the format can EXPRESS but slice 1 cannot HONOUR, as (field-prefix, JSON edit).
fn unhonourable() -> Vec<(&'static str, Value, &'static str)> {
    vec![
        (
            "soundfont",
            json!({"named": {"name": "MuseScore_General", "sha256": "ab"}}),
            "soundfont",
        ),
        ("voices", json!({"fixed": 4}), "voices"),
        ("voices", json!({"range": {"min": 2, "max": 6}}), "voices"),
        (
            "picture",
            json!([{"op": "luma_offset", "amount": -0.3}]),
            "picture",
        ),
        (
            "picture",
            json!([{"op": "luma_offset", "amount": 0.0}]),
            "picture",
        ),
        ("steps", json!("matched"), "steps"),
        ("seed", json!({"fixed": 114}), "seed"),
        (
            "mappings",
            json!({"builtin": "layer-reversed"}),
            "mappings.builtin",
        ),
        (
            "mappings",
            json!({"patched": {"base": "default", "merge_patch": {}}}),
            "mappings.patched",
        ),
    ]
}

/// L-28: an unhonourable field on a NON-default preset is refused as `Unsupported`, naming the
/// preset and the field, whether the preset is listed first or last (a refusal that depended on
/// order would let a candidate through by moving it).
#[test]
fn an_unhonourable_field_on_a_second_preset_is_refused_at_load_in_either_position() {
    for first in [true, false] {
        for (key, value, field) in unhonourable() {
            let got = with_second(first, |p| p[key] = value.clone());
            match got {
                Err(e @ PresetError::Unsupported { .. }) => {
                    let msg = e.to_string();
                    assert!(msg.starts_with("preset `second`:"), "{msg}");
                    assert!(msg.contains(&format!("`{field}")), "{field}: {msg}");
                    assert!(msg.contains("not supported by this build"), "{msg}");
                }
                other => {
                    panic!("{key}={value} (first={first}): expected Unsupported, got {other:?}")
                }
            }
        }
        let got = with_second(first, |p| p["provenance"]["status"] = json!("candidate"));
        assert!(
            matches!(got, Err(PresetError::Unsupported { ref field, .. }) if field.starts_with("provenance.status")),
            "candidate (first={first}): {got:?}"
        );
    }
}

/// The same fields on the DEFAULT preset are refused too (as Unsupported, or by the canonical
/// check) — never accepted.
#[test]
fn an_unhonourable_field_on_the_default_preset_is_refused() {
    for (key, value, _) in unhonourable() {
        let got = with_default(|p| p[key] = value.clone());
        assert!(
            matches!(
                got,
                Err(PresetError::Unsupported { .. }) | Err(PresetError::Invalid { .. })
            ),
            "{key}={value}: {got:?}"
        );
    }
    // A default that is honourable but carries settings is not the identity anchor: refused.
    let got = with_default(|p| p["settings"] = json!({"gain": "0.8"}));
    assert!(matches!(got, Err(PresetError::Invalid { .. })), "{got:?}");
}

/// The control for the two tests above: a second preset that changes only what slice 1 CAN honour
/// (a name, a description, a presettable knob, record-only provenance) is ACCEPTED. Without this,
/// "everything non-default is refused" would also pass them.
#[test]
fn an_honourable_second_preset_is_accepted() {
    let m = with_second(false, |p| {
        p["description"] = json!("A second test preset.");
        p["settings"] = json!({"gain": "0.8", "instruments": "6", "reverb": "off"});
        p["provenance"]["heard_on"] = json!({"soundfont": "bundled", "seed": 114});
        p["provenance"]["lineage"] = json!("default");
    })
    .expect("an honourable second preset loads");
    let second = m.presets.iter().find(|p| p.id == "second").expect("second");
    validate_against_cli(second).expect("its knobs are inside the bounds");
    let args = preset_render_args(second).expect("clap parses its knobs");
    assert_eq!(args.pipeline.instruments, 6);
    assert_eq!(args.audio.gain, 0.8);
}

/// `deny_unknown_fields` at every level of the manifest: a misspelt key is a refusal, not a
/// silently dropped intention.
#[test]
fn a_misspelt_key_anywhere_in_the_manifest_is_refused() {
    let edits: Vec<Edit> = vec![
        Box::new(|v| v["manifest_versoin"] = json!(1)),
        Box::new(|v| v["presets"][0]["sound_font"] = json!("bundled")),
        Box::new(|v| v["presets"][0]["provenance"]["stauts"] = json!("certified")),
        Box::new(|v| {
            v["presets"][0]["provenance"]["heard_on"] = json!({"soundfont": "x", "sed": 1})
        }),
        Box::new(|v| v["presets"][0]["mappings"] = json!({"builtin": "default", "extra": 1})),
        Box::new(|v| {
            v["presets"][0]["picture"] = json!([{"op": "luma_offset", "amount": 0.1, "x": 1}])
        }),
        Box::new(|v| v["presets"][0]["picture"] = json!([{"op": "blur", "amount": 0.1}])),
        Box::new(|v| v["presets"][0]["steps"] = json!("Matched")),
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut v = shipped();
        edit(&mut v);
        let got = parse_manifest(&v.to_string());
        assert!(
            matches!(got, Err(PresetError::Parse(_))),
            "edit {i}: expected a parse refusal, got {got:?}"
        );
    }
    // Manifest-level: wrong version, empty list, dangling default, duplicate id, bad id.
    let mut v = shipped();
    v["manifest_version"] = json!(2);
    assert!(matches!(
        parse_manifest(&v.to_string()),
        Err(PresetError::Manifest(_))
    ));
    let mut v = shipped();
    v["presets"] = json!([]);
    assert!(matches!(
        parse_manifest(&v.to_string()),
        Err(PresetError::Manifest(_))
    ));
    let mut v = shipped();
    v["default"] = json!("gone");
    assert!(matches!(
        parse_manifest(&v.to_string()),
        Err(PresetError::Manifest(_))
    ));
    assert!(with_second(false, |p| p["id"] = json!("default")).is_err());
    for bad in [
        "Upper",
        "under_score",
        "-lead",
        "trail-",
        "a--b",
        "",
        "x\u{1b}[31m",
    ] {
        assert!(
            with_second(false, |p| p["id"] = json!(bad)).is_err(),
            "id {bad:?}"
        );
    }
}

// ── PB-1 / PB-2 ─────────────────────────────────────────────────────────────────────────────

/// PB-1: EVERY render option the schema knows that is not in `PRESET_SETTING_IDS` is refused as a
/// preset setting — enumerated from the schema itself, so a row added later is covered without
/// editing this test. And every presettable id IS an Everyday/Advanced row (else a legitimate key
/// would be refused by `validate_against_cli`).
#[test]
fn pb1_every_non_presettable_schema_row_is_refused() {
    let schema = render_options_schema().expect("schema");
    for row in &schema {
        let id = row.id.clone();
        if PRESET_SETTING_IDS.contains(&id.as_str()) {
            assert!(
                matches!(row.tier, Tier::Everyday | Tier::Advanced),
                "presettable `{id}` must be an Everyday/Advanced row"
            );
            continue;
        }
        let got = with_second(false, |p| p["settings"] = json!({ id.clone(): "1" }));
        assert!(
            matches!(got, Err(PresetError::NotPresettable { ref setting, .. }) if *setting == id),
            "`{id}` must be refused as a preset setting: {got:?}"
        );
    }
    for id in [
        "midi-port",
        "output",
        "list-midi-ports",
        "help",
        "version",
        "zzz",
    ] {
        let got = with_second(false, |p| p["settings"] = json!({ id: "1" }));
        assert!(
            matches!(got, Err(PresetError::NotPresettable { .. })),
            "{id}: {got:?}"
        );
    }
}

/// PB-2: clap alone accepts NaN, infinities and negatives on several flags; a preset value outside
/// the window's bounds is refused by `validate_against_cli`. Controls: an in-bounds value passes.
#[test]
fn pb2_out_of_bounds_preset_values_are_refused() {
    let bad: &[(&str, &str)] = &[
        ("gain", "NaN"),
        ("gain", "inf"),
        ("gain", "-1"),
        ("gain", "1e9"),
        ("steps", "100000"),
        ("steps", "0"),
        ("thickness", "2"),
        ("thickness", "-0.5"),
        ("instruments", "0"),
        ("instruments", "99"),
    ];
    for (k, v) in bad {
        let m = with_second(false, |p| p["settings"] = json!({ *k: *v }));
        let refused = match m {
            Err(_) => true,
            Ok(m) => {
                let p = m.presets.iter().find(|p| p.id == "second").expect("second");
                validate_against_cli(p).is_err()
            }
        };
        assert!(refused, "preset `{k}` = {v} must be refused (PB-2)");
    }
    for (k, v) in [
        ("gain", "0.5"),
        ("steps", "10"),
        ("thickness", "0.3"),
        ("instruments", "6"),
    ] {
        let m = with_second(false, |p| p["settings"] = json!({ k: v })).expect("loads");
        let p = m.presets.iter().find(|p| p.id == "second").expect("second");
        validate_against_cli(p).unwrap_or_else(|e| panic!("{k}={v} is in bounds: {e}"));
    }
}

// ── L-30: the CLI flags ─────────────────────────────────────────────────────────────────────

/// L-30 / T-9: `--preset` and `--mappings` together are clap's ArgumentConflict on both
/// subcommands, in both orders, in both `--x v` and `--x=v` spellings.
#[test]
fn preset_and_mappings_are_mutually_exclusive_everywhere() {
    for sub in ["render", "play"] {
        for argv in [
            vec![
                "audiohax",
                sub,
                "--preset",
                "default",
                "--mappings",
                "m.json",
            ],
            vec![
                "audiohax",
                sub,
                "--mappings",
                "m.json",
                "--preset",
                "default",
            ],
            vec!["audiohax", sub, "--preset=default", "--mappings=m.json"],
            vec![
                "audiohax",
                sub,
                "x.png",
                "--mappings",
                "m.json",
                "--seed",
                "3",
                "--preset",
                "default",
            ],
        ] {
            let e = parse_with_explicit(argv.clone()).expect_err("conflict");
            assert_eq!(
                e.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{argv:?}: {e}"
            );
        }
        // Each alone parses.
        parse_with_explicit(["audiohax", sub, "--preset", "default"]).expect("preset alone");
        parse_with_explicit(["audiohax", sub, "--mappings", "m.json"]).expect("mappings alone");
        // Repeating either is clap's own refusal, not "last wins".
        assert!(parse_with_explicit([
            "audiohax", sub, "--preset", "default", "--preset", "default"
        ])
        .is_err());
        assert!(
            parse_with_explicit(["audiohax", sub, "--mappings", "a", "--mappings", "b"]).is_err()
        );
        // An unknown preset is clap's InvalidValue, listing the possible values.
        let e = parse_with_explicit(["audiohax", sub, "--preset", "nope"]).expect_err("unknown");
        assert_eq!(e.kind(), clap::error::ErrorKind::InvalidValue);
        assert!(e.to_string().contains("[possible values: default]"), "{e}");
    }
    // Not global, not on the other subcommands.
    for argv in [
        vec!["audiohax", "--preset", "default", "render"],
        vec!["audiohax", "analyze", "--preset", "default"],
        vec!["audiohax", "analyze", "--mappings", "m.json"],
        vec!["audiohax", "tui", "--preset", "default"],
        vec!["audiohax", "tui", "--mappings", "m.json"],
    ] {
        assert!(
            parse_with_explicit(argv.clone()).is_err(),
            "{argv:?} must not parse"
        );
    }
}

/// L-30 / T-4: the typed set is read from clap's `value_source`: a flag the user typed counts even
/// when its value equals the default (`--instruments 4`), in either spelling and in any position;
/// a flag not typed does not count; the positional image counts under its id.
#[test]
fn the_typed_set_comes_from_value_source() {
    for argv in [
        vec![
            "audiohax",
            "render",
            "x.png",
            "--instruments",
            "4",
            "--wav",
            "o.wav",
        ],
        vec!["audiohax", "render", "--instruments=4", "x.png"],
        vec!["audiohax", "play", "--instruments", "4"],
    ] {
        let (_, ex) = parse_with_explicit(argv.clone()).expect("parse");
        assert!(
            ex.contains("instruments"),
            "{argv:?}: typed-equal-to-default counts"
        );
        for untyped in ["steps", "thickness", "gain", "reverb", "expression", "v2"] {
            assert!(!ex.contains(untyped), "{argv:?}: `{untyped}` was not typed");
        }
    }
    let (_, ex) =
        parse_with_explicit(["audiohax", "render", "x.png", "--preset", "default"]).expect("p");
    assert!(ex.contains("image") && ex.contains("preset"));
    let (_, ex) = parse_with_explicit(["audiohax", "analyze"]).expect("analyze");
    assert!(!ex.contains("instruments"));
}

/// L-30 / §1 § 4.4 / T-6: the only `--help` change is the two flags under `render` and `play`
/// (long and short help), with the preset listed by clap's possible values. Top-level, `analyze`,
/// `tui` and every `modem` help carry neither flag.
#[test]
fn only_render_and_play_help_gain_the_two_flags() {
    let mut cmd = Cli::command();
    cmd.build();
    let long = |cmd: &mut clap::Command, path: &[&str]| -> (String, String) {
        let mut c = cmd.clone();
        let mut cur = &mut c;
        for p in path {
            cur = cur
                .find_subcommand_mut(p)
                .unwrap_or_else(|| panic!("subcommand {p}"));
        }
        (
            cur.render_long_help().to_string(),
            cur.render_help().to_string(),
        )
    };
    for sub in ["render", "play"] {
        let (l, s) = long(&mut cmd, &[sub]);
        for h in [&l, &s] {
            assert!(h.contains("--preset <NAME>"), "{sub}: {h}");
            assert!(h.contains("--mappings <FILE>"), "{sub}: {h}");
        }
        assert!(
            l.contains("- default: Default"),
            "{sub} long help lists the preset: {l}"
        );
        assert!(
            s.contains("[possible values: default]"),
            "{sub} -h lists the preset: {s}"
        );
    }
    let mut others: Vec<Vec<String>> = vec![
        vec![],
        vec!["analyze".into()],
        vec!["tui".into()],
        vec!["modem".into()],
    ];
    for s in cmd
        .find_subcommand("modem")
        .expect("modem")
        .get_subcommands()
    {
        others.push(vec!["modem".into(), s.get_name().to_string()]);
    }
    assert!(others.len() > 4, "the modem subcommands were enumerated");
    for path in others {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        let (l, s) = long(&mut cmd, &path);
        for h in [&l, &s] {
            // `modem encode` has its OWN, older `--preset <PRESET>` (a robustness preset); the
            // sound preset is `--preset <NAME>`.
            assert!(
                !h.contains("--preset <NAME>"),
                "{path:?} help must not gain --preset"
            );
            assert!(
                !h.contains("--mappings"),
                "{path:?} help must not mention --mappings"
            );
        }
    }
}

/// L-28 "refused at load, never ignored" read as a property of the PRESET, not only of a field:
/// a preset whose settings are individually valid but jointly unrenderable (`v2` without
/// `expression = on`, which compose refuses with "--v2 requires `--expression on`") passes BOTH
/// load-time gates today (`parse_manifest`, and the build-time `validate_against_cli`), so it would
/// ship and fail at every render, naming a flag the user never typed (§1 § 3.3 asked for a
/// "(from preset X)" suffix; not built). Latent in slice 1 (only `default` ships); owed before the
/// first non-default preset.
#[test]
fn f2_a_jointly_unrenderable_preset_is_refused_at_load() {
    let m = with_second(false, |p| p["settings"] = json!({"v2": "e3"}));
    let refused = match m {
        Err(_) => true,
        Ok(m) => {
            let p = m.presets.iter().find(|p| p.id == "second").expect("second");
            validate_against_cli(p).is_err()
        }
    };
    assert!(
        refused,
        "a preset that can never render was accepted at load"
    );
}

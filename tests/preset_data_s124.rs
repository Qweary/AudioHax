//! S124 Test Engineer (Specialist 5): the three candidate mappings files authored outside this
//! repository (`s124-preset-data/`: `layer-reversed`, `unpinned-ensemble`,
//! `darkened.NOT-YET-SHIPPABLE`) against the slice's bounds, and against composition.
//!
//! The files are NOT in this repository (they are candidates; none is in the manifest, L-28/L-40,
//! and the first release ships `default` only, D-50), so the test needs their directory in
//! `AUDIOHAX_PRESET_DATA_DIR`. It is `#[ignore]`d rather than "passes when unset": a test that
//! prints SKIPPED and reports `ok` counts as a pass in every summary line, which is the vacuous
//! green this file exists to avoid. Run it with
//! `AUDIOHAX_PRESET_DATA_DIR=<dir> cargo test --test preset_data_s124 -- --ignored`; with the
//! variable unset it FAILS, so it can never pass without checking something.
//!
//! What it checks, per file:
//! * the real chokepoint (`load_mappings_file`: bounded read + validation) accepts it;
//! * CONTROL: a copy with ONE tempo window inverted is refused (the validator fires on this data);
//! * it composes on every tracked picture in THIS (debug) build — debug assertions live, which is
//!   where L-37's `composition.rs` panic lived (8/22 images before §12's fix (d)) — and every piece
//!   is inside the 600 s cap (the security review addendum A.3 computed ≤ 445 s).
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use audiohax::mapping_loader::{load_mappings_file, MappingTable};
use std::path::{Path, PathBuf};

const FILES: [&str; 3] = [
    "layer-reversed.mappings.json",
    "unpinned-ensemble.mappings.json",
    "darkened.NOT-YET-SHIPPABLE.mappings.json",
];

fn data_dir() -> PathBuf {
    match std::env::var_os("AUDIOHAX_PRESET_DATA_DIR") {
        Some(d) => PathBuf::from(d),
        None => panic!(
            "AUDIOHAX_PRESET_DATA_DIR is not set: this test checks the candidate tables and must \
             not pass without them"
        ),
    }
}

fn compose_on_every_picture(name: &str, t: &MappingTable) {
    use audiohax::render::{prepare_play, ImageInput, RenderRequest, MAX_PIECE_MS};
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/images");
    let mut pics: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("images")
        .map(|e| e.expect("entry").path())
        .collect();
    pics.sort();
    assert!(pics.len() >= 6, "tracked pictures: {pics:?}");
    let (cli, _) = audiohax::presets::parse_with_explicit(["audiohax", "render", "--seed", "114"])
        .expect("parse");
    let audiohax::cli::Command::Render(args) = cli.command else {
        panic!("render")
    };
    let req = RenderRequest::from(&args);
    for pic in pics {
        let table = audiohax::mapping_loader::rebuild_mapping_table(t);
        let r = req.clone();
        let p2 = pic.clone();
        let got = std::panic::catch_unwind(move || {
            prepare_play(&r, ImageInput::Path(&p2), &table, &mut |_| Ok(()))
                .map(|plan| plan.timeline.iter().map(|e| e.at_ms).max().unwrap_or(0))
                .map_err(|e| e.to_string())
        });
        let pic = pic.display();
        match got {
            Err(_) => panic!("{name} on {pic}: PANICKED in compose (debug build)"),
            Ok(Err(e)) => panic!("{name} on {pic}: refused: {e}"),
            Ok(Ok(last)) => assert!(
                last > 0 && last <= MAX_PIECE_MS,
                "{name} on {pic}: last event at {last} ms"
            ),
        }
    }
}

#[test]
#[ignore = "needs AUDIOHAX_PRESET_DATA_DIR (the candidate tables live outside this repository)"]
fn candidate_tables_pass_the_chokepoint_and_compose_everywhere() {
    let dir = data_dir();
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("s124-preset-data-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("scratch");
    for f in FILES {
        let path: &Path = &dir.join(f);
        assert!(path.is_file(), "{} is missing", path.display());
        let t = load_mappings_file(path).unwrap_or_else(|e| panic!("{f}: {e}"));

        // CONTROL: the same data with one window inverted must be refused.
        let mut v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json");
        v["composition"]["affect"]["character_tempo"]["drone"] =
            serde_json::json!({"bpm_min": 48, "bpm_max": 8});
        let bad = scratch.join(format!("inverted-{f}"));
        std::fs::write(&bad, v.to_string()).expect("write");
        let e = load_mappings_file(&bad).expect_err("control: an inverted window must be refused");
        assert!(e.to_string().contains("character_tempo.drone"), "{e}");

        compose_on_every_picture(f, &t);
    }
}

//! tests/render_cli_determinism_s55.rs — S55 WS-4 SELF-SERVE `render --wav` DETERMINISM GUARD.
//!
//! PROPERTY VALIDATED: **self-serve render reproducibility under a fixed seed.** The exact
//! shipped `audiohax` binary, given the SAME image + the SAME `--seed`, must produce a
//! BYTE-IDENTICAL WAV — end-to-end, THROUGH COMPOSITION (seed → `set_composition_seed` →
//! `chord_engine::pick_progression` → the composed `CompositionPlan` → the absolute-ms
//! timeline → offline rustysynth render → WAV bytes). This is the guard that lets the
//! operator trust `render --wav … --seed N` renders reproducibly on their own machine.
//!
//! WHY BINARY-LEVEL (Option B of `docs/design-s55-ci-portability.md` §3.5, not the library
//! Option A): the composer's absolute-ms timeline builder `build_step_event_timeline` — the
//! step the SEEDED render path uses to turn the composed plan into events — is bin-private in
//! `src/main.rs` and is NOT exported by the library. So a library-level test cannot exercise
//! the full seed→composition→WAV chain the bin runs without re-implementing bin-private code,
//! which the design explicitly says NOT to do (§3.5 Option A "Notes": "prefer Option B rather
//! than moving code out of the bin"). Driving the real binary via `CARGO_BIN_EXE_audiohax`
//! exercises `run_render_wav` itself — the true shipped path, `--seed` register included.
//!
//! COMPLEMENTS existing coverage; it is NOT a duplicate of it (the precise gap named in §3.4):
//!   - `tests/seed_s41.rs` proves PLAN-level determinism (same seed ⇒ identical
//!     `CompositionPlan`) but explicitly NOT the WAV (`seed_s41.rs:16`).
//!   - `tests/ab_harness_s31.rs` proves RENDER-level byte-identity, but it holds the
//!     COMPOSITION CONSTANT — it renders one captured event list twice
//!     (`ab_harness_s31.rs:128`) and never exercises `set_composition_seed` → composed events.
//!
//! Neither chains seed → composition → WAV bytes end to end. This test does.
//!
//! HEADLESS + CI-SAFE: `render --wav` is offline — it opens no cpal/audio device (§3.1) — so
//! this runs on GitHub runners. It needs the bundled SoundFont (`assets/soundfonts/default.sf2`,
//! embedded via `include_bytes!` at build time), the SAME prerequisite every other
//! default-features test already carries (§1.5 CI fetch step covers it).
//!
//! Default feature set only (the binary needs the pure-Rust `synth` + `pure-analysis` defaults;
//! `--no-default-features` cannot build/run this, matching `ab_harness_s31.rs:11`).
#![cfg(all(feature = "synth", feature = "pure-analysis"))]

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Smallest committed image in the repo (`git ls-files` confirms it is tracked and NOT
/// git-ignored, so a fresh clone / CI checkout has it). Chosen for the FASTEST offline render:
/// the seed-determinism property under test is image-agnostic. `assets/images/example.jpg`
/// (the §3.5 example) works identically but renders ~2x slower.
const IMAGE: &str = "assets/images/AudioHaxImg1.jpg";

/// A small scan-step count passed via `--steps` to keep per-render wall-time down. NOTE: on the
/// COMPOSED render path the audible timeline length comes from the plan, not `--steps`
/// (`main.rs:420`), so this does NOT change the composition or weaken the determinism/difference
/// properties — it only shrinks the pure-analysis scan work (fewer bands to extract) that
/// dominates per-render time. Both same-seed↔identical and diff-seed↔different are empirically
/// confirmed to hold at this step count.
const STEPS: &str = "6";

/// Render `IMAGE` to a WAV with `--seed <seed>` via the REAL shipped binary and return the WAV
/// bytes. `tag` disambiguates the temp path so parallel test fns cannot collide (one test
/// binary ⇒ one pid, so pid alone is not unique across fns); a nanosecond stamp adds belt-and-
/// suspenders. The file is removed before returning. Asserts the process exited successfully.
fn render_seeded_wav_bytes(seed: u64, tag: &str) -> Vec<u8> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let mut wav_path = std::env::temp_dir();
    wav_path.push(format!(
        "audiohax_s55_render_{tag}_{}_{nanos}.wav",
        std::process::id()
    ));

    let status = Command::new(env!("CARGO_BIN_EXE_audiohax"))
        .args([
            "render",
            IMAGE,
            "--wav",
            wav_path.to_str().expect("utf-8 temp path"),
            "--seed",
            &seed.to_string(),
            "--steps",
            STEPS,
        ])
        .status()
        .expect("spawn audiohax binary");
    assert!(
        status.success(),
        "`audiohax render {IMAGE} --wav … --seed {seed}` must exit 0 (got {status})"
    );

    let bytes = std::fs::read(&wav_path)
        .unwrap_or_else(|e| panic!("read rendered WAV {}: {e}", wav_path.display()));
    let _ = std::fs::remove_file(&wav_path); // best-effort cleanup; a leaked temp is harmless.
    assert!(
        !bytes.is_empty(),
        "the rendered WAV must be non-empty (seed {seed})"
    );
    bytes
}

/// PRIMARY GUARD — same image + same `--seed` ⇒ BYTE-IDENTICAL WAV. This is the self-serve
/// reproducibility contract the operator depends on: two independent renders of the same seed,
/// each a fresh process (so nothing but the seed register carries state), must match byte-for-
/// byte through the whole seed→composition→WAV chain.
#[test]
fn test_render_wav_deterministic_same_seed() {
    let a = render_seeded_wav_bytes(7, "same_a");
    let b = render_seeded_wav_bytes(7, "same_b");
    assert_eq!(
        a.len(),
        b.len(),
        "same-seed renders must have identical byte length ({} vs {})",
        a.len(),
        b.len()
    );
    assert_eq!(
        a, b,
        "REGRESSION: same image + same --seed produced DIFFERENT WAV bytes — self-serve \
         `render --wav … --seed N` is no longer reproducible (a non-deterministic draw or a \
         wall-clock read has leaked onto the seeded composition/render path)."
    );
}

/// LOAD-BEARING-SEED GUARD — two DIFFERENT seeds ⇒ DIFFERENT WAV bytes. Proves the same-seed
/// identity above is NOT the trivial "output is constant regardless of seed" case: the seed
/// genuinely steers the composition (via `pick_progression`), so it actually reaches the WAV.
/// The 7-vs-8 pair is empirically confirmed to diverge on this image (and on `example.jpg`).
#[test]
fn test_render_wav_differs_by_seed() {
    let s7 = render_seeded_wav_bytes(7, "diff_7");
    let s8 = render_seeded_wav_bytes(8, "diff_8");
    assert_ne!(
        s7, s8,
        "seeds 7 and 8 must render DIFFERENT WAVs — if identical, the seed is NOT load-bearing \
         on the render path and the same-seed determinism guard proves nothing."
    );
}

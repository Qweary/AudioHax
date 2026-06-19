// src/lib.rs
// Library root so `src/bin/*.rs` bins can `use audiohax::modem;`
pub mod modem;

// Pure-Rust music subsystem, promoted into the library so it can be built and
// unit-tested headlessly via `cargo test --lib --no-default-features` (no system libs).
pub mod chord_engine;
pub mod mapping_loader;

// S41 — freeze-safe thread-local composition-seed register for the deterministic
// `--seed <u64>` feature. Threads a seed to the ONE RNG draw on the composition path
// (`chord_engine::pick_progression`) WITHOUT touching frozen `engine.rs` or any caller
// signature. Pure-Rust; builds & unit-tests under `--no-default-features`.
pub mod seed;

// S15 Slice 1 — the pure-Rust COMPOSER layer (form catalogue, plan, planner, StepContext).
// `--no-default-features`-clean: NO image type, NO OpenCV. Builds & unit-tests headlessly.
pub mod composition;

// WS-4 Phase 1 (S9) — pure-Rust shared core + CLI. Both build & unit-test under
// `cargo test --lib --no-default-features` (no system libs). The engine holds NO
// OpenCV/image/midir type; the CLI is the headless-testable clap grammar.
pub mod cli;
pub mod engine;

// WS-4 Phase 3 (S10) — pure-Rust ratatui TUI front-end, a pure OBSERVER over the S9
// engine seam. Builds & unit-tests under `cargo test --lib --no-default-features`.
pub mod tui;

// WS-4 Phase 2 (S11) Lane A — pure-Rust image+imageproc feature analyzer that
// implements engine::FeatureSource. Feature-gated so the bare
// `--no-default-features` lib (music + modem) stays dependency-free.
#[cfg(feature = "pure-analysis")]
pub mod pure_analysis; // Lane A — image+imageproc; implements engine::FeatureSource

// WS-4 Phase 2 (S11) Lane B — pure-Rust in-process synth sink (rustysynth+cpal) that
// implements engine::AudioSink. Feature-gated so the bare `--no-default-features`
// lib (music + modem) stays dependency-free.
#[cfg(feature = "synth")]
pub mod synth_sink; // Lane B — rustysynth+cpal; implements engine::AudioSink

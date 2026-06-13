// src/lib.rs
// Library root so `src/bin/*.rs` bins can `use audiohax::modem;`
pub mod modem;

// Pure-Rust music subsystem, promoted into the library so it can be built and
// unit-tested headlessly via `cargo test --lib --no-default-features` (no system libs).
pub mod chord_engine;
pub mod mapping_loader;

// WS-4 Phase 1 (S9) — pure-Rust shared core + CLI. Both build & unit-test under
// `cargo test --lib --no-default-features` (no system libs). The engine holds NO
// OpenCV/image/midir type; the CLI is the headless-testable clap grammar.
pub mod engine;
pub mod cli;

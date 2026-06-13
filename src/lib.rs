// src/lib.rs
// Library root so `src/bin/*.rs` bins can `use audiohax::modem;`
pub mod modem;

// Pure-Rust music subsystem, promoted into the library so it can be built and
// unit-tested headlessly via `cargo test --lib --no-default-features` (no system libs).
pub mod chord_engine;
pub mod mapping_loader;

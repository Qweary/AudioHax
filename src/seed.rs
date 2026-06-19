//! src/seed.rs — freeze-safe thread-local composition-seed register (S41).
//!
//! The deterministic-composition feature (`--seed <u64>`) threads a seed to the ONE
//! non-deterministic RNG draw on the composition path (`chord_engine::ChordEngine::
//! pick_progression`, the `thread_rng()` at chord_engine.rs:132) WITHOUT changing any
//! caller signature and WITHOUT touching frozen `src/engine.rs`.
//!
//! The seam (per `docs/design-s41-seed-feasibility.md`): callers (`engine.rs:409`,
//! `composition.rs:1558`) route through `pick_progression`, which constructs its RNG
//! internally. We give it a thread-local register it reads at call time:
//!
//! - register holds `Some(seed)` → `pick_progression` derives a per-call deterministic
//!   `ChaCha8Rng` keyed by `seed` mixed with a per-call counter, so a multi-section piece
//!   is reproducible AND its sections still diverge (each call advances the counter).
//! - register is `None` → `pick_progression` falls through to today's exact `thread_rng()`
//!   path, byte-unchanged. This is the DEFAULT (absent `--seed` ⇒ legacy behavior).
//!
//! The register is thread-local (`Cell`), so it is single-threaded-safe and needs no
//! locking; the composition path runs on one thread. Nothing here is `unsafe`.

use std::cell::Cell;

thread_local! {
    /// Active composition seed for this thread. `None` ⇒ legacy `thread_rng()` path.
    /// The companion counter advances on every seeded `pick_progression` call so the
    /// sequence of progression picks within one composition is reproducible but not
    /// all-identical (multi-section pieces stay distinct per section).
    static COMPOSITION_SEED: Cell<Option<u64>> = const { Cell::new(None) };
    static SEED_CALL_COUNTER: Cell<u64> = const { Cell::new(0) };
}

/// Set the active composition seed for this thread.
///
/// Call this ONCE at the freeze-safe entry (`main.rs` render/play handler), BEFORE the
/// composer/planner runs. `Some(seed)` makes the composition reproducible; `None` (the
/// default) preserves today's exact `thread_rng()` behavior. Setting the seed resets the
/// per-call counter so a fresh composition always starts from the same point.
pub fn set_composition_seed(seed: Option<u64>) {
    COMPOSITION_SEED.with(|c| c.set(seed));
    SEED_CALL_COUNTER.with(|c| c.set(0));
}

/// Read the active composition seed for this thread (used by `pick_progression`).
pub fn composition_seed() -> Option<u64> {
    COMPOSITION_SEED.with(|c| c.get())
}

/// Atomically (single-threaded) read-and-increment the per-call counter, returning the
/// value to mix into THIS call's RNG seed. The first seeded call gets `0`, the next `1`,
/// etc. — so each `pick_progression` invocation within one composition draws from a
/// distinct but fully deterministic sub-stream.
pub fn next_seed_call() -> u64 {
    SEED_CALL_COUNTER.with(|c| {
        let n = c.get();
        c.set(n.wrapping_add(1));
        n
    })
}

/// Derive the per-call 64-bit seed from the base composition seed and a call counter.
///
/// Counter-mixing scheme: `seed ^ (counter * ODD_GOLDEN)` where `ODD_GOLDEN` is the
/// odd 64-bit fixed point of the golden ratio (the Fibonacci-hashing multiplier). The
/// multiply spreads sequential counters (0,1,2,…) across the full 64-bit space and the
/// XOR folds the base seed in, so consecutive calls produce well-separated, uncorrelated
/// `ChaCha8Rng` streams — the same constant rust's `rand` uses for its seed mixing.
pub fn mix_seed(seed: u64, counter: u64) -> u64 {
    const ODD_GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
    seed ^ counter.wrapping_mul(ODD_GOLDEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_register_is_none() {
        // Fresh thread-local starts as None (legacy path).
        assert_eq!(composition_seed(), None);
    }

    #[test]
    fn set_and_read_roundtrips_and_resets_counter() {
        set_composition_seed(Some(42));
        assert_eq!(composition_seed(), Some(42));
        // Counter starts at 0 and advances.
        assert_eq!(next_seed_call(), 0);
        assert_eq!(next_seed_call(), 1);
        // Re-setting the seed resets the counter.
        set_composition_seed(Some(42));
        assert_eq!(next_seed_call(), 0);
        // Clearing returns to legacy.
        set_composition_seed(None);
        assert_eq!(composition_seed(), None);
    }

    #[test]
    fn mix_seed_diverges_per_counter() {
        let s = 12345u64;
        let m0 = mix_seed(s, 0);
        let m1 = mix_seed(s, 1);
        let m2 = mix_seed(s, 2);
        assert_eq!(m0, s, "counter 0 leaves the base seed unchanged");
        assert_ne!(m0, m1);
        assert_ne!(m1, m2);
        assert_ne!(m0, m2);
        // Different base seeds diverge at the same counter.
        assert_ne!(mix_seed(1, 5), mix_seed(2, 5));
    }
}

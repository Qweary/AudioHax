# Quality Gate Review — S41 Deterministic `--seed` Slice

**Reviewer:** Quality Gate (Specialist 6)
**Date:** 2026-06-19
**Changeset:** uncommitted, on top of committed `0974074`
**Verdict:** **PASS**

The deterministic-seed slice is purely additive, freeze-safe, and reproducible. Every claim in the work order was independently re-derived or re-run; nothing was taken on the strength of a prior report. No blocking issues. One non-blocking observation noted below.

---

## 1. Freeze Integrity

| Check | Result |
|---|---|
| `sha256sum src/engine.rs` | `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **MATCHES** the frozen keystone |
| `engine.rs` in changeset? | **NO** (`git diff --name-only` does not list it) |
| `composition.rs` in changeset? | **NO** (confirmed absent from diff) |
| Changeset scope | Exactly: `src/seed.rs` (new), `src/chord_engine.rs`, `src/cli.rs`, `src/lib.rs`, `src/main.rs`, plus `tests/seed_s41.rs` (new) and `docs/design-s41-seed-feasibility.md` (new). No other src/ file touched. |

The slice respects the freeze. `engine.rs` is byte-unchanged and merely *calls* `pick_progression`; the seam lives entirely in `seed.rs` + `chord_engine.rs`.

---

## 2. Additive-Safety Re-Derivation (the load-bearing claim)

**Claim:** absent `--seed`, the register is `None` and `pick_progression` takes the exact pre-slice `thread_rng` path, byte-unchanged.

**Pre-slice code (chord_engine.rs):**
```rust
let mut rng = thread_rng();
if let Some(p) = choices.choose(&mut rng) {
    return p.split('-').map(|s| s.to_string()).collect();
}
```

**Post-slice `None` arm:**
```rust
let picked = match composition_seed() {
    Some(seed) => { /* ChaCha8Rng path — only reached when --seed present */ }
    None => {
        let mut rng = thread_rng();
        choices.choose(&mut rng).cloned()
    }
};
if let Some(p) = picked {
    return p.split('-').map(|s| s.to_string()).collect();
}
```

**Re-derivation result: CONFIRMED behaviorally identical.** The `None` arm constructs the same `thread_rng()`, calls the same `SliceRandom::choose`, and consumes the iterator identically. The only mechanical difference is `.cloned()` on the `Option<&String>` so the two match arms share a return type — `choose` is unchanged, the draw is unchanged, and `.cloned()` does not touch the RNG. The subsequent `if let Some(p) = picked` is the same control flow as the original `if let Some(p) = choices.choose(...)`. The refactor to share the post-pick split did **not** alter the legacy draw. The "purely additive" guarantee holds.

This is corroborated empirically: two no-seed runs produced **different** WAVs (§3), i.e. the legacy non-deterministic `thread_rng` path is genuinely live when no seed is supplied.

---

## 3. Reproducibility — Re-Run Independently

`cargo run -- render assets/images/example.jpg [--seed N] --wav ...`, md5 of the WAVs:

| Run | md5 |
|---|---|
| `--seed 42` run 1 | `32efd672240b9b53199a7b31d60efa3f` |
| `--seed 42` run 2 | `32efd672240b9b53199a7b31d60efa3f` |
| `--seed 7` | `aca0b259e0e5dcc6d9dea389a92acbdf` |
| no-seed run A | `dd76ddaa8842bf834f4208af65aeee6a` |
| no-seed run B | `630d509c86789727558bb7098d237dec` |

- **Seed 42 ×2 → byte-identical.** Reproducibility works end-to-end through the WAV render.
- **Seed 7 → differs from seed 42.** The seed genuinely steers the draw.
- **Two no-seed runs differ from each other** (and from both seeded outputs). The legacy `thread_rng` path is preserved and non-deterministic, exactly as specified.

---

## 4. Full Verification Net

| Check | Result |
|---|---|
| `cargo build` (default) | **PASS** — only pre-existing, unrelated dead-code warnings in `modem_encode` / `unpack_tiled_payload` bins |
| `cargo test --lib --no-default-features` | **PASS** — 183 passed, 0 failed (incl. 3 `seed::tests` units) |
| `cargo test --test engine_equivalence` | **PASS** — 9/9 |
| `cargo test --test seed_s41` | **PASS** — 5/5 |
| `cargo test` (default, broad) **run 1** | **PASS** — 0 failures across all harnesses |
| `cargo test` (default, broad) **run 2** | **PASS** — 476 passed, 0 failed; no ordering flakiness |
| `cargo clippy` (seed.rs / chord_engine.rs / cli.rs) | **PASS** — no warnings on changed files |
| `cargo fmt -- --check` (changed files) | **PASS** — clean |

**Thread-local leakage check:** the broad `cargo test` suite was run **twice**. Both runs were fully green with no failures and no ordering-dependent flakiness. `cargo test` runs test fns in parallel across threads; the register is a per-thread `Cell`, so cross-test leakage cannot occur through it, and `seed_s41` belt-and-suspenders re-asserts `set_composition_seed(..)` at the start of every test (and before each independent run) which also resets the per-call counter. No leak observed.

---

## 5. Test-Quality Scrutiny (`tests/seed_s41.rs`) — not gamed

- **PT-SEED-1 (reproducible):** real whole-plan equality. Asserts both `progressions(first) == progressions(second)` AND full `CompositionPlan` `PartialEq`. Pre-S41 this could not hold (the draw was `thread_rng`), so it is a genuine determinism guard, not a tautology. Verified on two fixtures.
- **PT-SEED-2 (seed-sensitive):** real divergence, not trivially-always-true. The concrete 42-vs-7 `assert_ne!` is backed by a robust set-based leg over 7 seeds asserting ≥2 distinct realized sequences — guards against a coincidental single-pair collision. Could only pass if the seed actually steers the draw.
- **PT-SEED-3 (decorrelation):** meaningful. The strong leg asserts that within one seeded multi-section plan the per-section progressions are NOT all byte-identical (a dead/non-advancing counter would mix the same sub-stream every call → all-identical, failing this). Premise-checked (`sections.len() >= 3`). The weaker always-holding leg (multi-section reproducibility) is honestly labeled as the fallback.
- **PT-SEED-4 (legacy preserved):** correctly asserts validity/shape only, explicitly NOT determinism (because `thread_rng` is non-deterministic). It does not falsely assert determinism on the unseeded path. Re-asserts `None` per iteration.
- **PT-SEED-5 (freeze guard):** a real byte guard. Hashes `src/engine.rs` in-test with a dependency-free FIPS-180-4 SHA-256 and compares to the frozen constant. Complements the behavioral `engine_equivalence` 9/9 net with a bytes-level guard.

All five properties test real behavior. None reduce to `assert!(true)` or `is_ok()`.

---

## 6. `mix_seed` Soundness

`mix_seed(seed, counter) = seed ^ (counter.wrapping_mul(0x9E3779B97F4A7C15))` (the odd 64-bit golden-ratio / Fibonacci-hashing multiplier).

- **Counter 0 ⇒ mix == seed** — fine; the first call simply uses the base seed (asserted in `mix_seed_diverges_per_counter`).
- **Sequential decorrelation:** because the multiplier is **odd**, multiplication by it is a bijection on `u64` (invertible mod 2^64), so distinct counters `0,1,2,…` map to distinct products, and XOR with a fixed `seed` preserves distinctness. Therefore sequential calls produce **distinct, non-colliding** `ChaCha8Rng` seeds within one composition. The golden-ratio constant spreads consecutive integers widely across the 64-bit space (avalanche), so the resulting ChaCha8 streams are well-separated. Different base seeds at the same counter also diverge (asserted). This is correctness-grade (not crypto-grade, and the design doesn't claim crypto) — sequential calls decorrelate as required. **Sound.**

---

## 7. Thread-Local Correctness

- **Set before the planner, both paths:** `audiohax::seed::set_composition_seed(render_args.seed)` is placed in `run_render_wav` immediately after the image-understanding block and before `PipelineEngine::new` / the composer plan install. The identical pattern is placed in the `main()` play handler before the engine/plan build. **Confirmed set before the planner runs on both render and play paths.**
- **Counter reset on set:** `set_composition_seed` resets `SEED_CALL_COUNTER` to 0, so each fresh composition starts the per-call sub-stream sequence from 0. **Confirmed.**
- **Stale-register leak across invocations:** the CLI is one-shot (one process per invocation), so a cross-invocation leak is moot in production; within a process the register is explicitly set at each entry. No risk surfaced.

---

## Module Boundary Audit

- `seed.rs` (new): pure thread-local register + `mix_seed`; no image/MIDI/modem logic; no `unsafe`; `--no-default-features`-clean (its units run in the lib net). OK.
- `chord_engine.rs`: change confined to the RNG-acquisition seam inside `pick_progression`; no image processing, no MIDI output, no raw-feature references introduced. OK.
- `cli.rs`: adds an `Option<u64>` `--seed` arg to `PlayArgs` and `RenderArgs` only. OK.
- `main.rs`: orchestration only — sets the register before composing; no music-theory logic added. OK.
- `lib.rs`: `pub mod seed;` declaration only. OK.

No file was modified by an out-of-scope concern.

---

## Blocking Issues

None.

## Non-Blocking Observations

1. **Coverage scope (informational, not a defect):** `--seed` makes `pick_progression` deterministic, which (paired with the already-deterministic WAV render) yields byte-identical output — verified in §3. This is the only non-deterministic draw on the composition path per the design doc; if any future feature introduces a second `thread_rng`/entropy source on that path, it must route through the same register to preserve the reproducibility guarantee. No action needed now.

---

## Overall Verdict: **PASS**

Freeze intact (engine.rs + composition.rs byte-untouched), additive-safety re-derived and empirically corroborated, reproducibility confirmed by md5 (seed 42 ×2 identical; seed 7 differs; no-seed runs differ), full net green including two clean broad runs with no thread-local flakiness, tests validate real properties, `mix_seed` decorrelates soundly, and the register is set before the planner on both paths with counter reset on set. The slice is cleared to commit.

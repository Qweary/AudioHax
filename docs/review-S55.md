# Quality Gate Review — S55 (WS-4 Portability Verification + Self-Serve Render)

**Verdict: PASS WITH ISSUES** (both issues non-blocking; nothing blocks the commit)

Session class: OBJECTIVE / OUT-OF-CLASS. No music/taste dimension — STAGE 3 (musical logic) N/A;
no music-pipeline production files were modified. Acceptance is fully objective (build + tests + correct config).

Environment: cargo 1.96.0, default-features (pure-Rust) headless path.

---

## Compilation Status

- `cargo build --bins` → **OK** (`Finished dev profile ... target(s)`). All 9 binaries build clean under default features.

## Test Results

Full default-features suite, aggregated across all binaries:

- **Aggregate: 566 passed, 0 failed, 0 ignored.** BLOCKING check (any non-zero `failed`) — PASS.
  - lib unittests 252, main.rs unittests 14, bin unittests 0×7.
  - Integration test files: all green. Notable long-runners: modem_realair 80.6s, modem_cli_roundtrip 60.9s, rhythm_variety_s50 57.9s, variety_scorecard_s45 29.1s, qg_s53_review 27.9s, s52_probe_identity 27.8s.
  - Doc-tests: 0.
- **New file `tests/render_cli_determinism_s55.rs`: 2 tests, both included and GREEN** (`test result: ok. 2 passed; 0 failed`, 18.3–19.0s):
  - `test_render_wav_deterministic_same_seed` ... ok
  - `test_render_wav_differs_by_seed` ... ok
  - Runs the real shipped binary via `CARGO_BIN_EXE_audiohax`; observed 4 offline `render --wav` invocations at ~28.9–30.0s each internally (`--steps 6` keeps scan work small; timeline length comes from the composed plan, not `--steps`).

## Lint Status

- `cargo clippy --all-targets -- -D clippy::correctness` → **EXIT 0** (exact CI gate). PASS.
- Baseline note: tree carries ~41 pre-existing style/pedantic warnings by design; only the `correctness` class is gated, and it is green.
- **No net-new *correctness* warning introduced.** The new test file emits exactly **one** warning — `clippy::doc_lazy_continuation` at `render_cli_determinism_s55.rs:25` (default-`warn`, **style group, NOT correctness**) — which does not affect the gate. Logged as Non-Blocking #1.
- `cargo fmt -- --check`: the **new file is fmt-clean** (`rustfmt --check tests/render_cli_determinism_s55.rs` → exit 0). The only fmt drift reported is in `tests/qg_s53_review.rs`, a pre-existing committed file that is **not** part of the S55 change set (not in `git status`) — expected pre-existing drift, not flagged against S55.

## Freeze Verification

- `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **MATCHES** required hash. Engine frozen. PASS.

## Scope / Boundary Audit

- `git status -s` shows **exactly the three deliverables**, nothing else:
  - `?? .github/` (contains `.github/workflows/ci.yml`, NEW)
  - `?? docs/design-s55-ci-portability.md` (NEW, reference-only design doc, 24,666 bytes)
  - `?? tests/render_cli_determinism_s55.rs` (NEW)
- **No** production `src/*.rs`, `Cargo.toml`, `Cargo.lock`, `assets/*`, or unrelated test file is modified. PASS.
- The lead's revert of the stray cargo-fmt reflow of `tests/qg_s53_review.rs` is confirmed: `git status -s tests/qg_s53_review.rs` is empty (not modified).
- The new test file adds **TESTS ONLY** — no production code. It drives the shipped binary as a black box (`CARGO_BIN_EXE_audiohax`) and does not touch `engine.rs` behavior. Confirmed (plus `review-S55.md`, this QG doc, which I own).

## CI Workflow Correctness (`.github/workflows/ci.yml`)

- **Green default-features invocations only.** Runs `cargo build --bins`, `cargo test`, `cargo clippy --all-targets -- -D clippy::correctness`. **No `--no-default-features` anywhere** (the broken repo-wide path is correctly avoided). PASS.
- **SoundFont fetch is load-bearing and correctly positioned.** Verified the premise: `src/synth_sink.rs:47` embeds the SF2 via `include_bytes!("../assets/soundfonts/default.sf2")`; that path is `git check-ignore`-confirmed IGNORED and `git ls-files`-confirmed UNtracked, so a fresh checkout cannot compile without it on any of the three OSes.
  - Fetch step uses the **canonical URL** `https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2`, which **matches `assets/soundfonts/README.md:14` exactly**.
  - **URL is live: `curl -sIL … -w '%{http_code}'` → 200 today.**
  - Uses `shell: bash` (works on `windows-latest` via git-bash + curl).
  - Positioned **before** toolchain install and all build/test/clippy steps → all three OSes fetch before compiling. PASS. (An `actions/cache@v4` step precedes it; on a cache hit the file is restored, on a miss the curl runs — the file is present before build either way. `test -s` guards against a truncated/empty fetch.)
- **libasound2-dev is Linux-gated only** (`if: runner.os == 'Linux'`); Windows (WASAPI/WinMM) and macOS (CoreAudio/CoreMIDI) install no system deps. PASS.
- **Matrix:** `os: [ubuntu-latest, windows-latest, macos-latest]`, `fail-fast: false`, `timeout-minutes: 30`, `actions/checkout@v4`, `dtolnay/rust-toolchain@stable` with `components: clippy`, `Swatinem/rust-cache@v2`. All present and sane. PASS.
- **Clippy gate matches the honest baseline** (correctness-only; `-D warnings` correctly NOT used given the 41 pre-existing style warnings). PASS.
- **YAML well-formed:** `python3 -c "import yaml; yaml.safe_load(...)"` → OK.
- **Triggers:** `push` and `pull_request` on `master` (superset of the required `master` push trigger); adds a nice-to-have `concurrency` cancel-in-progress group.
- **Honest limits (expected, NOT defects):** runners have no audio device, so the cpal `play` path is never exercised — CI proves BUILD + headless/`render --wav` tests only; and the workflow cannot execute until the operator pushes to `origin` (auth-gated). Both are documented in the workflow header and acknowledged here.

## Test Quality Assessment

- **Meaningful property, not `assert!(true)`.** Two complementary guards:
  1. same image + same `--seed 7` (two independent fresh processes) ⇒ **byte-identical WAV** (`assert_eq!` on full byte vectors, plus a length pre-check).
  2. `--seed 7` vs `--seed 8` ⇒ **different WAV** (`assert_ne!`), which proves the seed is **load-bearing** on the render path — the same-seed identity is not the trivial "output constant regardless of seed" case.
- **End-to-end chain:** exercises the real shipped binary (`run_render_wav`) through seed → `set_composition_seed` → `pick_progression` → composed plan → offline render → WAV bytes. Genuinely complements existing coverage rather than duplicating it: `seed_s41.rs` stops at plan-level determinism (explicitly not WAV), and `ab_harness_s31.rs` holds composition constant. This is the first test chaining seed → composition → WAV bytes.
- **Temp/cleanup hygiene:** writes only under `std::env::temp_dir()`, disambiguated by tag + pid + nanosecond stamp (safe under parallel test fns); removes the file before returning (best-effort, leaked temp is harmless). Does not touch `engine.rs`.
- **Runtime:** ~18–19s for the pair (four internal renders at ~29s throttled by `--steps 6`), comparable to `variety_scorecard_s45` (~29s) — acceptable for an integration test.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **New-file clippy style warning (cosmetic).** `render_cli_determinism_s55.rs:25` trips `clippy::doc_lazy_continuation` ("doc list item without indentation") — a style-group lint, outside the gated `correctness` class, so the CI gate stays green. A one-line fix (indent the continuation or add a blank line in the module doc comment) would clear it. Not required for this commit.
2. **CI `curl` lacks `-f` (hardening suggestion).** The fetch step uses `curl -L -o … <url>` without `-f`/`--fail`. If the upstream URL ever returned an HTTP error, curl would write the (non-empty) error body to `default.sf2` and exit 0; `test -s` would pass and the build would `include_bytes!` a garbage SF2 that fails only at synth runtime. The URL is 200 today so there is no live impact; `curl -fL` (and/or a size/magic-bytes sanity check) would fail fast on a future upstream outage. Cosmetic robustness improvement, not a defect.

## Overall Verdict

**PASS WITH ISSUES.** All BLOCKING gates pass: build OK, 566/566 tests green (incl. the 2 new determinism tests), clippy correctness gate exits 0 with no net-new correctness warning, `engine.rs` sha256 frozen, and scope is exactly the three NEW deliverables with no production code touched. The CI workflow is correct on every audited dimension — green default-features invocations only, a correctly-positioned and genuinely load-bearing SoundFont fetch (canonical URL, live 200, `shell: bash`), Linux-gated ALSA deps, a sane 3-OS matrix, an honest correctness-only clippy gate, and well-formed YAML. The new determinism test validates a real, load-bearing property end-to-end and complements (does not duplicate) existing coverage. The two non-blocking issues are cosmetic (a style-lint line and a curl-hardening suggestion) and do not affect acceptance.

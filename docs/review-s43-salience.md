# Quality Gate Review — S43 Two-Tier Melody-Foregrounding Salience Fix

**Reviewer:** Quality Gate (correctness gate only; perceptual/taste gate runs separately, after this)
**Date:** 2026-06-19
**Work order:** `docs/design-s42-salience-diagnosis.md` §3
**Changes under review:**
1. `assets/mappings.json` — added `melody_forward` prominence profile; default `uniform`→`melody_forward`; escalation gate `fg_bg_contrast` floor `0.25`→`0.10`.
2. `tests/prominence_s43.rs` — 6 new property tests (NEW file).
3. No `src/*.rs` production code changed (Edit 3 chord_engine velocity bias deliberately NOT applied).

**Overall verdict: PASS (with one cosmetic non-blocking nit)**

---

## Compilation Status

`cargo build --release` — **PASS**. Finished in 15.18s. No errors. The two pre-existing warnings (`modem_encode`: 2, `unpack_tiled_payload`: 1) are in unrelated bin targets and are NOT attributable to this change (this slice touched no production source).

## Lint Status

- `cargo fmt -- --check` — **CLEAN** (exit 0, whole project; `rustfmt --check tests/prominence_s43.rs` also clean).
- `cargo clippy -- -W clippy::all` — **NO correctness warnings, NO errors.** All workspace clippy warnings are pre-existing lib/bin lints (modem, payload, channel_sim, etc.), unattributable to this change.
- `tests/prominence_s43.rs` emits **2 NEW clippy STYLE warnings** (`doc_lazy_continuation` at lines 371–372 — a multi-line `///` doc comment that clippy reads as a lazy blockquote continuation). These are documentation-formatting style nits, explicitly non-blocking per the work order. Listed under Non-Blocking Issues.

## Test Results (counts)

`cargo test` — **226 integration + 14 (lib default) + all bin/doc targets = 0 failures across the entire suite.**

Specifically confirmed (all PASS):
| Target | Count |
|---|---|
| `prominence_s43` | **6 / 6** |
| `prominence_s23` | **5 / 5** |
| `engine_equivalence` | **9 / 9** |
| `cargo test --lib --no-default-features` | **183 / 183** |
| `cargo test` (top integration aggregate) | **226 / 226** |

No test failed, was ignored, or was filtered unexpectedly.

## Boundary Audit

- `git diff --name-only` shows **only `assets/mappings.json`** among tracked non-test files. `git diff -- 'src/*'` is **empty** — no production source modified.
- `tests/prominence_s43.rs` is the only new test file under review; the `docs/design-s42-*.md` files are pre-existing untracked design inputs, not under review.
- **`src/engine.rs` is byte-frozen — CONFIRMED.** `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`, exactly equal to the required anchor. Re-confirmed independently by guard6 at test time.

## Data/Config Review

**Backward compatibility — PASS.** This is a content change, not a schema change. The new `melody_forward` row uses the SAME `LayerProminence` shape (`{role, weight}` pairs in a `layers` Vec) as the existing `uniform`/`subject_melody` rows — a purely additive Vec entry. The whole project builds and the full mapping-loader-driven suite passes, proving the new JSON parses under the current serde shape with no loader edit.

**Ranges — PASS.** All `melody_forward` weights ∈ [0,1] and sensible: Melody 0.78, CounterMelody 0.58, HarmonicFill 0.40, Pad 0.40, Bass 0.50; gate floor 0.10.
- Two-tier ordering verified against the actual loaded catalogue: `subject_melody.Melody (1.0) > melody_forward.Melody (0.78) > 0.5` neutral. ✓
- Bed band: Pad 0.40 and HarmonicFill 0.40 are both `< 0.5` (recede) and `> 0.25` (still support). Bass 0.50 is exactly neutral. ✓

**Freeze-safety — PASS (claim holds).** The work order §3 claim is that all edits are freeze-safe because the frozen engine only CONSUMES resolved weights and the `uniform`/`subject_melody` profiles are unchanged. Verified:
- `uniform` (`{}`, empty layers — the literal S42 root cause: empty → neutral 0.5 fallback) and `subject_melody` are byte-identical to before; only the `default` pointer and the gate floor changed.
- The only behavioral change is which profile a non-subject image RESOLVES to: default `uniform`→`melody_forward`; `example.jpg` escalates to the already-existing `subject_melody`.
- `engine_equivalence` 9/9 and `prominence_s23::prominence_neutral_is_byte_identical` + `engine_freeze_diff_empty` are green — the identity/legacy realizer path is unmoved. The kernel sha is unchanged. Confirmed.

## Test Quality Assessment

**Strong. The net checks meaningful musical properties through the real shipped resolution path — no `assert!(true)`, no hand-built profiles where the real planner should be used.**

- Guards 1, 2, 5 resolve prominence end-to-end through the SHIPPED pipeline: real reference images on disk (`example.jpg`, `Lena.png`) → `understand_image_pure` (real `subject_size`/`fg_bg_contrast`) → `CompositionPlanner::plan` (loaded from the real `assets/mappings.json`) → the per-section `orchestration.prominence` Vec the realizer consumes. This is genuine evidence the SelectTable gate fires (or not) on the real images, not on re-typed documented numbers.
- Guards 3, 4 read the two tiers straight from the loaded `prominence_catalogue`, with explicit magnitude pins so a future retune is a deliberate, visible change.
- **Guard 5 (velocity figure/ground) checks a SPECIFIC realized velocity comparison**, not just non-empty output: it drives the actual `realize_step` realizer with the prominence Vec resolved from each real image, builds a full 5-role ensemble, and asserts `Melody >= loudest non-melody role`, `Melody >= HarmonicFill` (the S42 culprit, called out explicitly), and a strictly-positive gap. Captured evidence: example gap=9.0 (Melody 102 vs 93), Lena gap=5.0 (Melody 98 vs 93) — the inverted S42 gap is corrected.
- Live evidence confirms the gate behavior on real features: example fg_bg_contrast=0.1355 ≥ 0.10 → subject_melody (1.0); Lena 0.0520 < 0.10 → melody_forward (0.78). Per-image divergence is real.
- Guard 6 is a non-duplicative belt-and-suspenders sha re-confirm, correctly deferring to the existing freeze authorities and degrading gracefully if `sha256sum` is absent.

No weak tests identified.

## Integration Assessment

No API changed; no type mismatch, missing import, or incomplete-integration TODO. The test file imports only existing public symbols (`chord_engine`, `composition`, `mapping_loader`, `pure_analysis`) and compiles clean. The data change is consumed by the existing planner with no code edit. Nothing partial.

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **(cosmetic)** `tests/prominence_s43.rs:371–372` — 2 NEW clippy `doc_lazy_continuation` style warnings on a wrapped `///` doc comment. Style-only, in a test file, non-blocking per the work order. Optional cleanup: re-indent the continuation or it can be left as-is.

## Overall Verdict

**PASS.** Build green, full suite green (all named targets at expected counts), `src/engine.rs` byte-frozen, the change is additive-data-only with the freeze-safety argument verified, and the new test net checks real musical properties through the shipped resolution path. The single non-blocking item is a cosmetic doc-comment lint. The perceptual/taste assessment is a separate gate that runs after this correctness gate.

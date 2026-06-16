# Quality Gate Review — Slice K2a (S26)

**Role:** Quality Gate (independent verifier). **Date:** 2026-06-16.
**Repo HEAD:** `9cd96816787daaf2135ca79f333bbc49b95f64ce` (uncommitted K2a working tree).
**Slice under review:** K2a — per-region affect + generalized multi-excursion planner + §4.1(i)
planner harmony re-root. Realizer (`engine.rs` / `chord_engine.rs`) byte-frozen.

**VERDICT: PASS.**

Every claim verified independently from the code and the build. The realizer is byte-frozen
(sha + 9/9 equivalence). The module boundary holds. The per-region direction logic matches the
endorsed input doc including the 0.40 cut-point correction. The tests are real and property-specific
(not gamed). Codename-clean. One honest scope caveat (the two-excursion path is unreachable until
K2b routing) is correctly K2b/K3 scope per design §6 and does NOT block K2a — the single-excursion
path is genuinely hearable today.

---

## 1. Compilation

`cargo build` (default features): **PASS.** Finished clean. The only warnings are pre-existing
`unused_variable`/`dead_code` in unrelated modem bins (`modem_encode.rs`, `unpack_tiled_payload.rs`);
none in the changed files.

## 2. Lint

`cargo clippy`: **PASS (non-blocking style only).** Zero `error`-level diagnostics (grep count = 0).
All warnings are style lints (`needless_range_loop`, `len_zero`, `manual_div_ceil`, etc.), the
majority pre-existing in the lib and the modem bins. No correctness (`clippy::correctness`) warnings.
Style lints are NON-blocking per the validation contract.

`rustfmt --edition 2021 --check` on the four changed files
(`src/composition.rs src/pure_analysis.rs tests/keyplan_s25.rs tests/keyplan_k2a.rs`): **PASS**
(exit 0, no diff). Bare `cargo fmt` was NOT run.

## 3. Test Results

`cargo test`: **PASS — all green, zero failures, zero ignored.**

Every kickoff-named net confirmed green:

| Net | Result | Net | Result |
|---|---|---|---|
| lib unit (incl. composition/pure_analysis mods) | 163 ok | engine_seam | 10 ok |
| engine_equivalence | **9 ok** | cli_parse | 24 ok |
| composition_s15 | 5 ok | tui_render | 13 ok |
| diversity_s13 | 10 ok | phase2_pure_pipeline | 7 ok |
| keyplan_s25 | 11 ok | modem_roundtrip | 17 ok |
| keyplan_k2a | **9 ok** | modem_realair | 10 ok |
| saliency_s18 | 12 ok | texture_s17 | 7 ok |
| figuration_s20 | 8 ok | affect_s22 | 8 ok |
| prominence_s23 | 5 ok | qg_probe_band_isolation | 1 ok |

main.rs unit 5 ok; bin unit nets 0 ok (no tests, as expected). No `FAILED`, no `ignored`.

## 4. Byte-Freeze Verification (the load-bearing audit)

**PASS — the realizer is frozen, witnessed three ways:**

1. `sha256sum src/engine.rs` =
   `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23` — **MATCHES** the anchor.
2. `git diff HEAD -- src/engine.rs src/chord_engine.rs` — **EMPTY** (both untouched). `chord_engine.rs`
   sha unchanged at `b448d9363499234e7e5ddce18fbb3017b754acbea3af51126cd5e51b1215e39b`.
3. `cargo test --test engine_equivalence` — **9/9 byte-green**; the full golden sweep
   (`test_full_golden_sweep_is_byte_identical`), the cadence/velocity/hold golden, and the
   role-pitch golden all pass — goldens 240/114/84/36/79 unmoved.

`git status --short` shows ONLY the claimed files modified: `M src/composition.rs`,
`M src/pure_analysis.rs`, `M tests/keyplan_s25.rs`, plus untracked `tests/keyplan_k2a.rs` (the new
net) and the S26 design/input docs. The known stray `assets/images/magicstudio-art.jpg` is UNTRACKED
and **not staged** (verified). `assets/mappings.json` `git diff` is **EMPTY** — confirmed NOT changed,
as claimed.

## 5. Module Boundary Audit

**PASS.**

- **`pure_analysis.rs` stays pixels-in / image-free-scalars-out.** `band_affect` reads only
  `RegionStats.mean_value` / `RegionStats.dominant_hue` (already produced by `analyze_regions_pure`)
  and returns `(f32, f32)`. NO new pixel pass, NO new dependency, NO music/MIDI/modem logic. The
  circular hue mean (unit-vector `atan2`) is correct and handles the red wrap; degenerate band falls
  back to the caller's whole-image values. Verified by the 5 new `band_affect` unit tests
  (means / subject-exclusion / circular-wrap / degenerate-fallback / determinism).
- **`composition.rs` planner consumes scalars, performs NO image extraction.** `region_excursion_offset`
  and `resolve_key_scheme` name no pixel type; they read the `ImageUnderstanding` scalar fields only.
- **The harmony re-root is a CALL change, not a `chord_engine` body edit.** Confirmed at the
  `generate_chords` call site: `let section_root_midi = (home_root_midi as i16 + section_offset as
  i16).clamp(0,127) as u8;` is passed where the literal `home_root_midi` used to go. `chord_engine.rs`
  diff is empty.

## 6. Musical / Logic Review

**PASS — matches the endorsed input doc (`input-s26-k2a-region-direction.md`).**

- **`region_excursion_offset` direction mapping** is exactly the endorsed predicate:
  `hue_dist >= 60.0 → relative_offset` (±3); else `valence > 0.40 → +7` (dominant); else
  (`<= 0.40`) → `+5` (subdominant). Menu strictly `{+7,+5,+3,−3}`. The **0.40 cut-point correction**
  (the deliberate fix of review-S25 note 1, which had shipped a 0.50 split) is applied — constants
  `LOW_VALENCE_MAX = 0.40`, `HIGH_VALENCE_MIN = 0.60`, HIGH+MID both → +7, only LOW → +5, with the
  boundary inclusive at 0.40 (LOW). Mode-family-aware relative: major-family → −3, minor-family → +3,
  via the existing `relative_offset`. Hue distance is measured against `subject_hue` (per-region
  generalization), wrap-aware on the 0..360 circle.
- **Whole-image fallback reproduces K1 exactly.** The `excursion_offset` shim builds a whole-image
  `RegionAffect { valence: affect_valence, hue: secondary_hue }` and delegates to
  `region_excursion_offset`. The unit test `region_excursion_reproduces_k1_on_whole_image` sweeps
  valence × hue × mode and asserts byte-equality with the shim — the generalization invariant holds.
- **`resolve_key_scheme`:** energy-DESCENDING rank (`bg.energy > fg.energy → [bg,fg]`, else `[fg,bg]`
  with a stable foreground-first tiebreak); per-rank region read via `ranked.get(rank)`; rank beyond
  the two real regions → whole-image fallback (total, no panic). Resolution policy applied LAST:
  `Resolve` forces `offsets[n-1] = 0` (Invariant A), `Open` leaves it. `None`/empty (`home_only`) and
  unknown rules → all-zero (the byte-freeze identity). The `≤2-distinct-non-home-keys` cap is
  structurally guaranteed (two ranked regions, menu math) and tested (`at_most_two_distinct_non_home_keys`).
- **The re-root is real.** Offset 0 → `home_root + 0` → identical chord roots → byte-identical (so the
  byte-freeze argument is sound); a non-zero offset shifts every chord note by exactly that offset,
  proven deterministically by `harmony_reroots` against the public RNG-free `generate_chords`.

## 7. Test Quality

**PASS — the tests are genuine, not gamed.**

- **The `keyplan_s25.rs::valence_direction` FIX is NOT a loosening.** The assertions are unchanged in
  musical intent: `b_hi == 7`, `b_lo == 5`, `b_hi != b_lo`. What changed is the *driver*: valence is
  now steered through the rank-0 region's OWN `background_brightness`/`foreground_brightness`, and the
  near path is held by pinning both region hues to `subject_hue` (so the per-region hue read does not
  spuriously trip the ≥60° relative branch). This is the correct adaptation to the per-region read
  landing — it still validates direction-from-affect, now per-region. Not gamed.
- **The 9 `keyplan_k2a.rs` tests each assert a SPECIFIC property** with concrete value assertions; no
  `is_ok()`/non-empty-only checks. Highlights:
  - `energy_descending_rank` swaps the two regions' energies and asserts `(b1,c1) == (c2,b2)` —
    proving rank is energy-driven, not slot-driven (the strongest test in the net).
  - `harmony_reroots` asserts every chord note (incl. root `notes[0]`) shifts by exactly the offset AND
    that offset 0 is byte-identical — a real harmony re-root witness.
  - `resolution_policy` drives the SAME affect through Resolve vs Open and asserts the final offset
    diverges (forced-0 vs kept) — the policy is the sole variable.
  - `home_only_byte_zero`, `per_region_fallback_reproduces_k1`, `parse_offset_rule_grammar`
    (incl. unknown→home) all assert exact offsets.
  - The net respects RNG-boundary discipline: harmony content is asserted ONLY through the
    deterministic `generate_chords`, never through `plan()`'s `pick_progression`.

## 8. Integration + Hearability Assessment

**Independently confirmed:** `assets/mappings.json` `key_scheme` SelectTable has exactly ONE rule —
`fg_bg_contrast >= 0.25 → aba_excursion`, default `home_only`. The catalogue contains `abac_rondo`
(the two-excursion `region_related:c` scheme) but **NOTHING in the selector ever picks it** → it is
UNREACHABLE by any real image today. The Test Engineer's flag is **correct.**

**Is this a K2a blocker? No — it is correctly K2b scope.** Design §6 explicitly scopes the routing
rules (the new catalogue rows + the `key_scheme` SelectTable rules that reach the multi-excursion
schemes) to slice **K2b**. K2a's stated scope is the planner + pure_analysis machinery and the
single-excursion path; it does not claim to ship the two-excursion routing. No over-claim.

**What a REAL image actually produces differently than K1 today:** for any image with
`fg_bg_contrast >= 0.25`, the planner selects `aba_excursion` = `[home, region_related:b, home]` on a
3-section form (`rounded_binary` / `ternary_aba` / `theme_and_variations`). The single B excursion now:

- **(a) reads per-region affect direction** — B travels by the rank-0 (more-energetic non-subject)
  region's OWN brightness and OWN hue-vs-subject, NOT the whole-image `affect_valence`/`secondary_hue`.
  Two images with identical whole-image valence but different foreground/background brightness now go
  to DIFFERENT B keys. This is a genuine, live behavioral change vs K1 (where B read whole-image only),
  proven by `keyplan_s25::valence_direction` and `keyplan_k2a::per_region_fallback_reproduces_k1`.
- **(b) re-roots the harmony** — the section's chord ROOTS (not just the theme melody's tonic) now
  travel by the offset, via the §4.1(i) planner re-root, proven byte-exactly by `harmony_reroots`.

So **K2a IS independently hearable for the single-excursion case**: the B section's chords genuinely
move to the per-region-chosen key, where under K1 only the melody leaned over home harmony. The
two-excursion ("eye sweeps twice") case and the smooth pivot/landing wait for K2b routing + K3 — which
is the correct, honest slicing per the design. The machinery for the two-excursion case is fully
present and unit-proven (`distinct_excursions`, `energy_descending_rank` drive `abac`-shaped schemes
directly); it is simply not yet routed by shipped data.

## 9. Blocking Issues

**None.**

## 10. Non-Blocking Issues

1. **Cosmetic doc/code fallback mismatch (pure_analysis.rs).** `band_affect`'s degenerate-band hue
   fallback is documented in `composition.rs` as `secondary_hue`, but the runtime caller in
   `understand_image_pure` passes `dominant_hue` as the fallback. This is unreachable in practice (a
   single argmax over a 3×3 grid can never make all 4 band cells the subject, so `n` is never 0), so
   it has zero behavioral effect. Cosmetic only.
2. **`abac_rondo` is unreachable by shipped routing (K2b scope).** Stated above — not a K2a defect;
   the routing rules are explicitly K2b. Flagged so the lead tracks that K2b must add the
   `key_scheme` rule + catalogue rows to make the two-excursion path firing.
3. **Pre-existing scheme/form length-mismatch debug witness (NOT a K2a regression).** The shipped
   3-section `aba_excursion` scheme, when applied to a 4/5-section form (`aaba`/`abac`/`abbac`), only
   covers the first 3 roles; for `aaba` (Statement/Statement/Contrast/Return) the row 1
   `region_related:b` lands on a Statement (home role), which trips the `debug_assert_eq!`
   role-alignment witness in DEBUG builds (never panics in release, never affects output — the offset
   still resolves via the same math). This is pre-existing K1 routing behavior (mappings.json
   untouched by K2a); K2a only widened the Coda allowance. Worth K2b reconciling when the per-form
   schemes land. No action required for K2a.
4. **Style clippy lints** across the lib/bins (non-blocking) — could be swept opportunistically but
   are out of K2a scope.

## Overall Verdict: **PASS**

K2a delivers exactly what it claims: a byte-SAFE planner + pure_analysis generalization that makes the
single B excursion read per-region affect and re-root the harmony, with the realizer frozen and proven
frozen. The multi-excursion catalogue/routing and the smooth pivot are correctly deferred to K2b/K3.
The slice is independently hearable, the tests are real, and the boundary holds. Safe for the lead to
integrate.

# Review S27 — BUILD K2b (Quality Gate)

**Slice:** K2b — byte-safe planner/data slice making multi-excursion key plans REACHABLE
(the K2a dead-data fix), shipping the first Open scheme (OFF by default), making
`resolves_home` conditional on `ResolutionPolicy`, and reconciling two K2a non-blocker nits.
**HEAD:** `336c66a` (K2a). **Surface verified:** `assets/mappings.json`, `src/composition.rs`
(staged) + `tests/keyplan_k2b.rs` (untracked, NEW).

## OVERALL VERDICT: PASS

All ten checks pass on independently re-derived evidence. The realizer is byte-frozen, the
order-isomorphic routing is sound (zero debug_assert panics in a debug build over the full net),
the operator lock holds (the one Open scheme is unrouted and a live tripwire test proves it), and
no codename leaks. No blockers. Two minor carry-forward observations, neither blocking.

---

## CHECK 1 — BYTE-FREEZE (central safety property): PASS

- **`src/engine.rs` sha256:** `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`
  — matches the anchor exactly.
- **`git diff --cached --stat`:** ONLY `assets/mappings.json` (+38/−3) and `src/composition.rs`
  (+14/−3); `tests/keyplan_k2b.rs` is untracked (NEW). No locked file appears.
- **Locked-set status check** (`engine.rs`, `chord_engine.rs`, `midi_output.rs`, `synth*`,
  `cli.rs`, `tui.rs`, `main.rs`, `modem.rs`, `bin/*`, `lib.rs`): EMPTY — none modified.
- **`cargo test --test engine_equivalence`:** 9/9 byte-green (the full golden sweep
  `test_full_golden_sweep_is_byte_identical` passes — the net passing IS the witness that goldens
  240/114/84/36/79 are conceptually unmoved).
- **`realize_step` public signature:** byte-identical worktree vs `HEAD` (same 7-param signature
  at the same line; the S23 prominence threading remains an additive-PRIVATE param, signature
  unchanged).

## CHECK 2 — THE DEAD-DATA FIX (the slice's reason to exist): PASS

Independently inspecting the SHIPPED `key_scheme.rules` in `mappings.json`: there are now six
active routing rules, one per multi-excursion scheme. `abac_rondo` — the row the K2a QG flagged as
unreachable — is now selected by `vertical_emphasis >= 0.6 AND fg_bg_contrast >= 0.25`. Every new
scheme (`rounded_binary_excursion`, `ternary_aba_excursion`, `aaba_excursion`, `abac_rondo`,
`abbac_excursion`, `theme_and_variations_resolve`) has a firing rule. The k2b net's six
`routing_reachability_*` tests drive the SHIPPED planner (no steering) and confirm each fires its
intended scheme with a non-zero interior excursion. `abac_rondo` is no longer dead data.

## CHECK 3 — ORDER-ISOMORPHIC DIVERGENCE + T&V RESOLVE TWIN: SOUND (not scope creep)

**(a) Genuinely order-isomorphic?** YES. Extracting both ladders from the shipped data:

| # | `form` rule (pick / predicates) | `key_scheme` rule (pick / predicates) |
|---|---|---|
| 1 | theme_and_variations ← complexity≥0.66, edge≥0.6 | theme_and_variations_resolve ← complexity≥0.66, edge≥0.6, **fg_bg≥0.25** |
| 2 | ternary_aba ← quadrant_contrast≥0.6 | ternary_aba_excursion ← quadrant_contrast≥0.6, **fg_bg≥0.25** |
| 3 | aaba ← aspect≥1.6, palette_bimodality≤0.3 | aaba_excursion ← aspect≥1.6, palette_bimodality≤0.3, **fg_bg≥0.25** |
| 4 | abac ← vertical_emphasis≥0.6 | abac_rondo ← vertical_emphasis≥0.6, **fg_bg≥0.25** |
| 5 | abbac ← edge≥0.7, value_key≥0.6 | abbac_excursion ← edge≥0.7, value_key≥0.6, **fg_bg≥0.25** |
| 6 | (default rounded_binary) | rounded_binary_excursion ← **fg_bg≥0.25** (catch-all) |

Same order, same predicates per rank, each scheme rule AND-ing in the shared `fg_bg_contrast>=0.25`
subject gate. Isomorphic.

**(b) Every jointly-reachable (form, scheme) pair lands `region_related:*` offsets on NON-home
roles?** YES — verified by mapping each form's section roles against its twin scheme's offset rules:

- theme_and_variations [Statement, Development, Development] × T&V_resolve [home, region:b, region:c] — region rules on Development roles. OK
- ternary_aba [Statement, Contrast, Return] × [home, region:b, home] — region on Contrast. OK
- aaba [Statement, Statement, Contrast, Return] × [home, home, region:b, home] — region on Contrast. OK
- abac [Statement, Contrast, Return, Coda] × [home, region:b, home, region:c] — regions on Contrast + Coda. OK
- abbac [Statement, Contrast, Contrast, Return, Coda] × [home, region:b, region:c, home, region:c] — regions on Contrast/Contrast/Coda. OK
- rounded_binary [Statement, Contrast, Return] × [home, region:b, home] — region on Contrast. OK

All `home` rules land on Statement/Return; all `region_related:*` on Contrast/Development/Coda. The
debug_assert at `composition.rs:~1472` cannot trip for any routed pair. **Empirical confirmation:**
the FULL net was run in a DEBUG build (debug_assert active) — keyplan_k2b 14/14 and the entire
348-test default net pass with ZERO panics.

**(c) Is `theme_and_variations_resolve` legitimate or scope creep?** LEGITIMATE. `input-s27` §4
explicitly judges T&V the one form that MAY end open, but §2 locks the Open T&V routed OFF by
default. The form-axis rule for a T&V-classified image must therefore route to a *Resolve* scheme,
or T&V images would have no excursion vocabulary at all (falling back to home_only). The Resolve
twin is the structurally-aligned, ends-home companion that the form rule actually selects; the Open
twin is the unrouted showcase. This is exactly the design contract, not an addition beyond it. The
order-isomorphic divergence from the spec's original ordering is the correct call: it is what keeps
every selectable (form, scheme) pair role-aligned so the safety assert stays a real guard rather
than a tripped invariant.

## CHECK 4 — OPERATOR LOCK (off-home endings OFF by default): PASS

`theme_and_variations_excursion` is the ONLY `resolution:"open"` row in the catalogue and is
selected by NO routing rule (the T&V form rule routes to the *_resolve* twin). The k2b test
`no_routed_image_ends_off_home` drives the SHIPPED routing over one firing fixture per active rule
(1)-(6) and asserts `resolved().last() == 0` for ALL six, AND that each fired a real non-zero
excursion (so "ends home" is a true recapitulation, not a vacuous all-home pass). It is a live
tripwire — it fails loudly if a future edit ever routes an Open scheme. Lock holds.

## CHECK 5 — CONDITIONAL resolves_home + OPEN WITNESS: PASS

Test semantics match the Music Theory contract:
- `resolve_schemes_land_home` iterates the SHIPPED catalogue, filters to `Resolve`, asserts
  `last()==0` for each (>=7 checked: home_only, aba_excursion, the four binary/ternary/aaba/abac
  excursions, abbac, T&V_resolve). The legacy rows (home_only, aba_excursion) default to `Resolve`
  via `#[serde(default)]` (confirmed in `composition.rs:500-503`), so they are covered.
- `open_schemes_may_end_off_home` asserts the Open final ∈ {+7,+5,+3,−3,0} across a 3-case sweep,
  PLUS a deliberate-feature witness: a firing image whose rank-1 region drives V2 to a NON-ZERO
  menu value (`assert_ne!(final_off, 0)`). NOT vacuous — it exercises a genuine off-home final.
- Tests are REAL (concrete `assert_eq!`/`assert_ne!`/`assert!` over resolved Vec<i8> values; no
  `assert!(true)` anywhere in the file).

## CHECK 6 — THE TWO NITS: PASS

- **(A) debug_assert clarified, NOT weakened.** The diff adds a K2b explanatory comment only; the
  assertion code is unchanged — still `debug_assert_eq!(rule_is_home, role_is_home, ...)` inside the
  per-section loop. It still fires on a future misaligned routing (the comment itself documents the
  pre-K2b case it would catch). Not a no-op.
- **(B) field-doc names the runtime value, zero behavior change.** The `foreground_hue` /
  `background_hue` doc comments changed from "defaults to `secondary_hue`" to naming the actual
  degenerate-band fallback `dominant_hue` that `understand_image_pure` passes. Doc-comment-only; no
  computed value moved (confirmed: the diff touches only `///` lines + the assert comment block; the
  9/9 engine_equivalence goldens are unmoved).

## CHECK 7 — TEST QUALITY: PASS

The 14 tests check meaningful properties with real fixtures. `firing_distinct_regions()` is a
hand-crafted deterministic fixture (no `thread_rng`); the file documents the RNG-boundary
discipline (offsets are pure-resolved BEFORE any RNG runs and are the only thing asserted —
chords/Roman numerals are never asserted). `at_most_two_distinct_non_home_keys` genuinely bounds
abbac (the 5-section stress case) across a 4-case region sweep (same/distinct/relative), asserting
`distinct.len() <= 2`. Routing reachability is driven through the SHIPPED `mappings.json` via the
public `plan()` (the real contract), not a hand-built scheme. `smooth_keys_only` runs a full nested
cross-product (8 schemes × 4 region cases × 2 home modes = 64 combos) with a non-vacuity guard
(`nonzero_seen > 0`).

## CHECK 8 — FULL NET + HEADLESS TRUTH: PASS

- **Full default net (`cargo test`):** 348 passed, 0 failed across all binaries/integration tests
  (lib 163, main 5, plus integration suites incl. keyplan_k2b 14, keyplan_k2a 9, keyplan_s25 11,
  engine_equivalence 9; modem real-air + roundtrip green).
- **`cargo test --lib --no-default-features`:** 128 passed, 0 failed — matches the Test Engineer's
  report.
- **`cargo build --no-default-features` (bin):** FAILS with 4 errors (unresolved
  `audiohax::pure_analysis`, `midir`, `synth_sink`) sourced in LOCKED files (`modem.rs`, `main.rs`,
  `bin/modem_encode.rs`). The only K2b-changed-file reference in the output is a pre-existing
  dead-code WARNING for `excursion_offset` at `composition.rs:1368` (outside the K2b diff hunks,
  which are at lines ~99-103 and ~1465-1474). The break is PRE-EXISTING (feature-gating in locked
  code, S11-era) and is NOT chargeable to K2b — K2b touched no locked file and no error originates
  in its diff.

## CHECK 9 — CODENAME SCRUB: CLEAN

Grepped all three changed/new code files plus the two new design docs against the full internal
codename set. One apparent match was a false positive: a three-letter substring inside an ordinary
English adverb meaning "at the same time" on `composition.rs:391` — a line OUTSIDE the K2b diff. No
genuine codename leak in any of the five files (or in this review document).

## CHECK 10 — SCOPE & OVERCLAIM: IN-SCOPE, NOT OVERCLAIMED

The slice stayed strictly within K2b: catalogue + routing DATA (`mappings.json`), two planner nits
+ one clarifying comment (`composition.rs`), and tests. NO realizer touch (engine.rs byte-frozen;
chord_engine.rs / midi_output.rs / synth* unchanged). Nothing is overclaimed.

**What K2b honestly does NOT do** (correctly disclosed in `input-s27` §2 / the test header):
- There is no K3 pivot/cadence machinery yet. Under a routed multi-excursion Resolve form, section
  B travels but the final section (C/Coda/Return) is FORCED home by policy — there is no smooth
  pivot into or out of the excursion.
- The Open scheme, if it were ever realized, would arrive at and leave its off-home key ABRUPTLY (a
  splice, not an intentional drift) because no pivot/land-home cadence exists until K3. This is
  precisely why the Open scheme ships present-in-data but routed OFF — the policy is proven
  end-to-end and the conditional test lands, but no real image hits it until it sounds intentional.

---

## BLOCKERS

None.

## CARRY-FORWARD NITS (non-blocking)

1. **N-K2b-1 (informational):** `excursion_offset` (`composition.rs:1368`) is now dead code under
   `--no-default-features` (a warning, not an error; pre-existing pattern). Worth a glance in a
   future cleanup slice to confirm it is still reachable under the default feature set or prune it.
2. **N-K2b-2 (forward-looking):** the Open-ending abruptness is a deferred-to-K3 audible gap, not a
   defect. When K3 lands the pivot/cadence machinery, re-listen to a routed Open T&V before
   un-gating it (the design's locked plan; tracked here so it is not lost).

---

**BOTTOM LINE FOR THE LEAD:** PASS — K2b is byte-safe (engine.rs sha intact, 9/9 equivalence,
diff = data + planner-nits + new tests only), the order-isomorphic routing is sound with the Resolve
T&V twin a legitimate design move, the operator lock holds on a live tripwire, full net 348/0;
merge it.

# Review S25 — Quality Gate record for slice K1 (image→home-key + ABA returning structural key plan)

**Role:** Quality Gate — runs LAST, re-verifies independently, trusts no build/test report.
**Slice:** K1 — fill the already-threaded `key_scheme` / `key_offset_semitones` spine so a
subject/ground image LEAVES home for its B section to a closely-related key and RETURNS home; a
subjectless field stays home (byte-stable). Planner + loader + data only; realizer untouched.
**Repo:** working tree at `/home/qweary/working/audiohax-engagement/AudioHax`, HEAD `ea99165`.
**Toolchain:** cargo 1.96.0 (`$HOME/.cargo/bin`).
**Date:** 2026-06-16.

---

## VERDICT: PASS

All five criteria independently verified PASS. The byte-freeze holds exactly (sha256 unmoved, the
three freeze-locked files have an empty diff, goldens 240/114/84/36/79 are the asserted literals).
The K1 behavior is real, reachable, menu-correct, and mode-family-correct. The three-touch mirror is
wired on all three touch-points and the round-trip witness genuinely bites the loader-side mirror.
The full headless suite is green with zero regressions; keyplan_s25 is 11/11. The two test
adaptations are sound, honest, and design-sanctioned, not gamed.

Two NON-BLOCKING observations are recorded for the operator's re-listen bench (one is a real numeric
deviation from the Music Theory input doc; one is the accepted per-region-affect limitation). Neither
moves a golden, neither touches the realizer, and neither blocks the slice. They are re-listen
tunables, surfaced loudly below so the operator hears them before the ear test.

---

## Criterion 1 — BYTE-FREEZE (engine_equivalence byte-green + sha256 frozen): **PASS**

Independently gathered evidence (not from the build report):

- `sha256sum src/engine.rs` →
  `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23` — **matches** the locked
  witness in the spec §3/§7 exactly.
- `git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs` → **EMPTY**. None
  of the three freeze-locked files moved a byte.
- The goldens in `tests/engine_equivalence.rs` are the asserted LITERAL values, read directly (not
  silently changed): hold **240** (`d_hot.events[0].hold_ms, 240` — `1.20*200`), bass velocities
  **114** (sat100, `round(96+18)`) and **84** (sat0, `round(96−12)`), bass note **36**
  (`G_BASS_NOTE = 36`, C2 floor), melody note **79** (`G_MELODY_NOTE = 79`, G5-area). All five
  literals present and unchanged.
- **Feature-config note (asked for explicitly):** `cargo test --test engine_equivalence
  --no-default-features` **does NOT compile** — the bin targets (`main.rs`, `midi_output.rs`,
  `synth_sink`) require the `synth`/`midi` features (`midir` unresolved, `synth_sink` gated). The
  net therefore runs under **DEFAULT features**, where `cargo test --test engine_equivalence` is
  **9/9 PASS** (`test_full_golden_sweep_is_byte_identical`, `test_cadence_velocity_and_hold_golden`,
  `test_role_pitch_bass_below_melody_golden`, + 6 others, all ok). I report the working command
  rather than the spec's parenthetical.

No criterion-1 failure mode is present. PASS.

---

## Criterion 2 — The K1 behavior is real and reachable: **PASS**

Read `resolve_key_scheme` / `excursion_offset` / `relative_offset` / `lookup_key_scheme` and the §2.4
wiring in `src/composition.rs`:

- **The un-discard (~:951):** `let key_scheme_id = self.plan_mappings.key_scheme.select(u);` — the
  previously-discarded `_key_scheme_id` is now bound and used.
- **The resolve (~:971):** `let offsets = resolve_key_scheme(lookup_key_scheme(&self.plan_mappings.
  key_scheme_catalogue, &key_scheme_id), &form_spec.sections, u, &home_mode);` — once per plan, after
  the form and home_mode are chosen, mirroring the S23 prominence resolve.
- **The section write (~:1077):** `key_offset_semitones: offsets.get(i).copied().unwrap_or(0),` —
  replaces the literal `0`.
- **The spine (~:1092):** `let key_scheme = offsets.clone();` — replaces `vec![0i8; sections.len()]`.

**A firing image (fg_bg_contrast ≥ 0.25) genuinely produces a non-zero B and a home A/final.** The
shipped `key_scheme` rule fires `aba_excursion` on `fg_bg_contrast ≥ 0.25`; `aba_excursion` has
`A:home / B:region_related:b / A:home`. `resolve_key_scheme` maps `home → 0` and
`region_related:* → excursion_offset(u, home_mode)`. Tests §5.3/§5.4 confirm the FINAL and all home
roles resolve 0 while B is non-zero, on both K1 forms (`rounded_binary`, `ternary_aba`).

**Menu math hand-derived independently (the +5/−5 trap is the load-bearing check):**

| B-region condition | `excursion_offset` result | `tonic_pc = (60+off)%12` | Pitch class | Correct? |
|---|---|---|---|---|
| high valence (`affect_valence ≥ 0.5`), near hue (<60°) | **+7** dominant | 7 | G | ✔ dominant of C |
| low valence (`< 0.5`), near hue | **+5** subdominant | 5 | F | ✔ subdominant of C — **and NOT −5** |
| strong contrast (hue dist ≥ 60°), major-family home | **−3** | 9 | A | ✔ relative minor of C major |
| strong contrast, minor-family home | **+3** | 3 | D♯/E♭ | ✔ relative major of C minor |

The **−5 regression the spec/input doc warned against is avoided**: the code hardcodes `5` (not `−5`)
on the subdominant branch; `(60−5)%12 = 7 = G` would have silently collapsed IV into V, and it does
not. `relative_offset` is mode-family-computed from `home_mode` (substring match on
aeolian/minor/dorian/phrygian/locrian → `+3`, else `−3`), so it is major→−3 / minor→+3 as required
and composes with the hue-selected mode; the unknown-mode arm falls to `−3`, matching the realizer's
Ionian fallback. The hue distance is wrap-aware (`raw % 360`, then `360 − raw` if `> 180`). All four
menu values land in the allowlist `{+7, +5, +3, −3}`. PASS.

---

## Criterion 3 — The three-touch mirror is wired (the silently-breakable arm): **PASS**

- `key_scheme_catalogue` is on **`PlanMappings`** (`composition.rs:721`, `#[serde(default)]`), on
  **`CompositionMappings`** (`mapping_loader.rs:155`, `#[serde(default)]`), and in the
  **`From<CompositionMappings> for PlanMappings`** impl (`composition.rs:742`,
  `key_scheme_catalogue: c.key_scheme_catalogue`). All three touch-points present.
- **The round-trip witness bites the real failure mode.** Test §5.11
  (`key_scheme_catalogue_round_trips`) loads the SHIPPED `assets/mappings.json` through
  `load_mappings → PlanMappings::from`, then asserts (a) the loaded
  `pm.key_scheme_catalogue` is non-empty and carries `aba_excursion` with its `region_related` B
  rule, and (b) a firing image resolves a NON-ZERO offset through `plan()`. This is a true
  non-`home_only` resolve through the real load path — not the home_only path — so it bites where
  §5.1 (which passes either way) cannot.
- **Stronger than the prompt assumed on the From arm.** The `From` impl is an explicit struct
  literal with NO `..Default::default()`, so deleting the `key_scheme_catalogue` arm is a
  **compile error** (E0063 missing field), not a runtime silent-drop. The realistic silent-drop
  failure mode is the **loader-side** omission: drop the field from `CompositionMappings` and serde
  defaults it to empty, the From arm carries empty, and §5.11's `!is_empty()` / non-zero-resolve
  assertions FAIL at runtime. So both touch-points are witnessed — loader omission → test failure,
  From-arm omission → build failure. The Risk-6 arm genuinely bites. PASS.

---

## Criterion 4 — All nets green + no-overclaim: **PASS**

Independently run: `cargo test` (default features — the only config that compiles, per criterion 1).
Per-suite counts read from the runner output:

| Suite | Count | | Suite | Count |
|---|---|---|---|---|
| lib (unit) | 151 | | engine_seam | 10 |
| main (unit) | 5 | | figuration_s20 | 8 |
| affect_s22 | 8 | | **keyplan_s25** | **11** |
| cli_parse | 24 | | modem_realair | 10 |
| composition_s15 | 5 | | modem_roundtrip | 17 |
| diversity_s13 | 10 | | phase2_pure_pipeline | 7 |
| engine_equivalence | 9 | | prominence_s23 | 5 |
| saliency_s18 | 12 | | qg_probe_band_isolation | 1 |
| texture_s17 | 7 | | tui_render | 13 |

**Every suite passed; ZERO failures, zero ignored.** keyplan_s25 = **11/11**. Every prior net the
prompt named is present at its expected count and unregressed: engine_equivalence 9, engine_seam 10,
composition_s15 5, diversity_s13 10, prominence_s23 5, affect_s22 8, figuration_s20 8, saliency_s18
12, texture_s17 7, cli_parse 24, tui_render 13, phase2_pure_pipeline 7, and the modem nets
(roundtrip 17 + realair 10). (The empty bin-unittest targets and `qg_probe_band_isolation` 1 are the
remaining harness targets — all green.)

**Backward-compatibility of `assets/mappings.json` verified independently:** the new
`key_scheme_catalogue` field carries `#[serde(default)]` on **BOTH** structs — `PlanMappings`
(`composition.rs:720–721`) and `CompositionMappings` (`mapping_loader.rs:154–155`). An old config
without the field deserializes to an empty Vec → only `home_only` reachable → byte-stable. PASS.

---

## Criterion 5 — The two test adaptations are SOUND, not gamed: **PASS**

### (a) §5.8 `energy_ordered_b_region` adapted because there is no per-region affect field — **TRUE, sound, honest.**

I read the full `ImageUnderstanding` struct (`composition.rs:39–98`). It carries per-region ENERGY
(`subject_energy`, `foreground_energy`, `background_energy`) but **NO per-region valence, hue, or
brightness**. Direction is whole-image `affect_valence`; non-subject hue is the whole-image
`secondary_hue`. The limitation is a real working-tree fact, exactly design Risk 3.

The adapted test is honest: it (1) flips ONLY the energy inequality and confirms BOTH orderings
resolve a valid, in-menu, non-zero B offset (the region-pick branch is exercised under both), and
(2) carries a tripwire `assert_eq!(plan_bg.key_tempo.key_scheme, plan_fg.key_tempo.key_scheme)`
asserting the two orderings resolve the SAME value in v1 — documented as the EXPECTED v1 behavior, so
a future per-region-affect sub-slice that makes them diverge will trip this and force a deliberate
revisit. It deliberately does NOT claim the offset diverges by region (which would be a false test).
This is the correct, non-gamed adaptation of §5.8.

### (b) §5.11 loads the shipped mappings.json rather than inline JSON — **TRUE, sound.**

The reason is real: `mapping_loader::GlobalMappings` (loaded via the shipped file) has required,
non-`#[serde(default)]` fields (`saturation_to_harmonic_complexity`, `feature_normalization`, etc.),
so a stripped inline JSON would track the schema and rot. Loading the authoritative shipped file is
exactly what the production planner does, and the test still bites the mirror (criterion 3). Sound,
not a shortcut.

**Neither adaptation weakens the slice's CLAIMS** — except that §5.8's energy-order claim is wired
but not yet audibly differentiated per-region (the RE-LISTEN note below states this loudly).

---

## HONESTY / NO-OVERCLAIM (the operator has the ear)

- **The audible win is validated in LOGIC and TESTS ONLY.** The tests prove the per-section
  `key_offset_semitones` and the `KeyTempoPlan.key_scheme: Vec<i8>` take the intended non-zero values
  on a firing image and return to 0 on the recap, and that the register invariant holds across all
  new offsets. They do **NOT** establish that the music *sounds* better. The actual ear test
  (different images → audibly different keys; ABA departs and returns satisfyingly; the direct
  modulation does not sound like a splice) is **operator-owned and POST-session**. This review makes
  no claim about how it sounds.

### RE-LISTEN notes for the operator

1. **(NON-BLOCKING — real deviation from the Music Theory input doc.)** `excursion_offset` splits
   high-vs-low valence at a single `affect_valence >= 0.5` cut. The Music Theory input
   (`docs/input-s25-k1-keyplan-harmony.md` §3a/§3c) specifies a THREE-band table with
   `τ_hi = 0.60`, `τ_lo = 0.40`, and a MID band that goes to the dominant (+7) — i.e. the doc's
   net +5/+7 boundary sits at **0.40**, the implementation's at **0.50**. So images with valence in
   **[0.40, 0.50)** get **+5 (subdominant)** in the build where the doc would give **+7 (dominant)**.
   This is a pure-data routing tunable (it changes WHICH closely-related key a near, mid-dark image
   takes; it never produces an off-menu offset, never moves a golden, never touches the realizer).
   It is sound and shippable as-is, but it is a genuine seed deviation the operator should hear on
   the bench — if the mid-dark images feel too settled/plagal, restoring the 0.40 boundary (or the
   full three-band table) moves them back to the dominant lift the doc intended.

2. **(NON-BLOCKING — design Risk 3, accepted.)** Because per-region affect is not first-class, the
   energy-order picks the region LABEL (B = the more-energetic of background/foreground energy) but
   the **offset is whole-image-driven**. Two images with the same whole-image valence/hue but a
   different fg/bg energy split get the **SAME B offset** in v1. The "energy-ordered region → key"
   claim is structurally WIRED but not yet audibly differentiated per-region. The §5.8 tripwire will
   notice when per-region affect lands. Surface this on the re-listen: if the ear wants the
   energetic region to actually steer the key color, the minimal prerequisite is a small
   `pure_analysis` add (per-region brightness/hue), flagged in the spec as an optional sub-slice, not
   a K1 blocker.

3. **(NON-BLOCKING — Music Theory's own re-tune candidates, zero-golden-risk pure-data tunables.)**
   Per `input-s25-k1-keyplan-harmony.md` §6: `τ_contrast` 60.0° → consider **45.0°** (route more
   contrasting B-regions to the smooth 7/7-shared relative); narrow the neutral valence band
   (0.55/0.45) so subdominant/relative earn more airtime; and the low-valence subdominant-vs-relative
   swap (relative may be the deeper shadow for Laments). The implementation currently uses
   `τ_contrast = 60.0` (matches the seed) and the single 0.50 valence cut (note 1). All are
   re-listen knobs; none block. The menu offsets themselves (`+7, +5, +3, −3`) and the
   `relative_offset` mode-family rule are common-practice facts, NOT re-tune candidates.

---

## Issues

- **Blocking:** none.
- **Non-blocking 1:** the valence-cut deviation (RE-LISTEN note 1) — `>= 0.5` vs the input doc's
  three-band 0.40/0.60. Affects routing in the [0.40, 0.50) valence band only; pure-data, no golden
  risk. Recommend the operator confirm the boundary on the re-listen bench (and/or the build team
  reconcile the code to the input doc if 0.40 was intended).
- **Non-blocking 2:** per-region affect limitation (RE-LISTEN note 2) — accepted design Risk 3,
  tripwire-guarded.

---

## Discipline confirmation

This Quality Gate wrote ONLY `docs/review-S25.md`. No source, test, or asset was modified. No
throwaway verification script was created (the menu math was hand-derived above; the mirror bite was
established by reading the explicit struct-literal `From` impl, not by mutation). Working tree after
this review carries only the slice's own changes (`assets/mappings.json`, `src/composition.rs`,
`src/mapping_loader.rs` modified; `docs/input-s25-k1-keyplan-harmony.md`, `tests/keyplan_s25.rs`,
`assets/images/magicstudio-art.jpg` untracked) plus this review doc — no stray file from the gate.
Commit serialization is the orchestrator's; the gate does not commit.

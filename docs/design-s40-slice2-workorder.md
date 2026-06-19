# S40 — Slice-2 BUILD WORK ORDER: per-image HOME (Finding #1)

**Author:** Rust Architect (design-only; no `src/*`, `assets/*`, or `tests/*` edited this session)
**Governing design:** `docs/design-s38-synthesis.md` §4 (Slice-2 shape), §5 DP-2 (driver), §6 (freeze ledger); `docs/design-s38-theory.md` §2.2 (register headroom / safe window [57,68]); `docs/design-s38-aesthetics.md` §1 (hue→pitch-class identity).
**Operator decision baked in (DP-2, NOT re-opened):** driver = **Option 1: hue → pitch-class (identity)**. Home derived from dominant hue → chromatic pitch class → seated into the safe register band [57,68]. valence→register (Option 2) and build-both (Option 3) are OUT of scope. No valence in the home derivation.

## FREEZE-SAFETY VERDICT (read first): **SAFE.**
No path in this slice opens or moves `src/engine.rs` (byte-frozen sha256 `e50c7db1…48261`) and no golden fixture moves. The single locus is `composition.rs:1302` (plan-time). Full reasoning in §7. **No FREEZE-BREAK.**

---

## 1. Current-state analysis (confirmed line refs + downstream-flow trace)

### 1.1 The locus (confirmed live)
- **`src/composition.rs:1302`** — `let home_root_midi = 60; // C4 seed (EngineConfig.root_midi default); A/Return stay home.` — CONFIRMED. This is THE line to replace.
- **`src/composition.rs:1294–1301`** — `u.dominant_hue` is already read here via `lookup_range_map(&mappings.global.hue_to_mode, u.dominant_hue)` for `hue_mode`; `valence_family_mode(...)` then consumes `u.affect_valence`. CONFIRMED: both `u.dominant_hue` and `u.affect_valence` are in scope on `u`. **No new feature threading is needed.**
- **`src/composition.rs:1484`** — `home_root_midi` is written into `KeyTempoPlan { home_root_midi, .. }`. This is the single carrier downstream.

### 1.2 The downstream-flow trace (does anything assume home==60?)
`home_root_midi` flows from `:1302` into the `KeyTempoPlan` spine (`:1484`) and is consumed in exactly two ways. Every consumer was read live:

**(a) Section re-root (composition.rs:1416–1441).** `section_root_midi = (home_root_midi as i16 + section_offset as i16).clamp(0,127) as u8` (`:1418`). The section root is `home + key_offset_semitones`. `generate_chords(.., section_root_midi, ..)` (`:1425`) and `tonic_triad(section_root_midi, ..)` (`:1441`) build at this root. **Moving the home base shifts EVERY section's root uniformly by the same delta** — this is exactly correct: the whole piece transposes to the new key, excursion offsets remain relative. The clamp to `0..=127` holds for any home in [57,68] plus the menu offsets `{+7,+5,+3,−3}` (max 68+7=75, min 57−3=54 — both valid MIDI). **Nothing here assumes home==60.**

**(b) Realizer tonic-pc reductions (chord_engine.rs).** Every realizer consumer of `ctx.key_tempo.home_root_midi` reduces it to a **pitch class** before use:
  - `:1635`, `:1659` (walking/pedal bass): `tonic_pc = (home_root_midi + key_offset_semitones).rem_euclid(12)`.
  - `:2723` (`theme_pitch`): `tonic_pc = (home_root_midi + key_offset_semitones).rem_euclid(12)`, then `degree_to_pitch(degree, tonic_pc, mode, floor)` where `floor = MELODY_REGISTER_FLOOR + bright_lift`.
  - `:2802`, `:2917`, `:2993` (pivot/cadence helpers): `home_root_pc = home_root_midi % 12`.

  **The decisive register fact (verified):** `degree_to_pitch` (`:2511`) and `theme_pitch` re-seat the resolved pitch class via `seat_pc_in_register(pc, floor)` (`:1282`), which places the pc at-or-above the **role floor**, independent of `home_root_midi`'s absolute octave. So moving the home **rotates pitch classes; it does NOT move absolute register** — the bass does not boom, the melody does not shriek. This is why Finding #1 is register-safe where a naive root change would not be (Theory §2.1 confirmed in code).

**Verdict:** `home_root_midi` flows downstream cleanly. No consumer assumes the literal value 60; every consumer takes either a uniform transposition (a) or a `% 12` pitch class (b). Theory's "NO pivot/cadence rework needed" reading is confirmed: the home is one fixed center per piece; `key_scheme` excursions are a separate, already-guarded relative axis (`resolve_key_scheme` / `key_scheme_handle` at `:1310–1314` read `home_root_midi` only via the section re-root arithmetic in (a), never as an absolute assumption).

### 1.3 The existing optional-block idiom (the pattern this slice reuses)
`assets/mappings.json` already carries optional `composition` sub-blocks (`texture`, `figuration_catalogue`, `prominence`, `key_scheme_catalogue`, `affect`, …), each gated by **`#[serde(default)]`** on `CompositionMappings` (mapping_loader.rs:110–162). The whole `composition` block is itself `Option<CompositionMappings>` (mapping_loader.rs:172–173). **The home block follows this exact idiom: a new `#[serde(default)]` optional field → absent → planner falls back to 60.** This is the same back-compat floor every prior composition slice used.

The range-map idiom is also already present and is the natural lookup primitive: `lookup_range_map(map, value) -> Option<String>` (mapping_loader.rs:245) returns `None` when no range key matches `value` — the built-in absence-of-mapping hook. `mappings.global.hue_to_mode` (mappings.json:3–10) is the canonical example of a `"lo-hi": value` range map keyed on a hue-domain quantity.

---

## 2. The hue→pitch-class + seating algorithm (signatures + algorithm — NO bodies)

### 2.1 Conceptual pipeline
```
u.dominant_hue (f32, degrees 0..360)
        │  hue → chromatic pitch class  (MUSIC THEORY fills the cuts)
        ▼
   pc: u8 (0..=11)            // 0 = C, 1 = C#, … 11 = B
        │  seat pc into the safe register band [57,68]
        ▼
   home_root_midi: u8         // GUARANTEED ∈ [57,68] by construction (GR-2)
```

### 2.2 Lookup signature (Implementer-owned, composition.rs)
A private planner helper that resolves the home, with absence → 60. Proposed signature (no body):

```rust
/// Resolve the per-image home root MIDI from the dominant hue, seated into the safe
/// register band. Returns 60 (C4) when the optional home block is absent — the
/// defensive fallback that reproduces today's behavior byte-for-byte.
fn resolve_home_root_midi(
    home: Option<&HomeRootMap>,   // the optional mappings.json home block; None => 60
    dominant_hue: f32,            // u.dominant_hue, degrees 0..360
) -> u8;                          // GUARANTEED ∈ [57,68] when home is Some, else exactly 60
```

Behavior contract (algorithm, not code):
1. If `home` is `None` → return `60`. (Defensive fallback, §1 / Invariant INV-4.)
2. Else resolve `pc = lookup_pc(home, dominant_hue)`; if the lookup yields no match → return `60` (treat an unmatched hue exactly like an absent block — same byte-for-byte floor; never panic).
3. Else return `seat_pc_in_band(pc)`.

Call-site replacement at `composition.rs:1302`:
```rust
// BEFORE: let home_root_midi = 60; // C4 seed …
// AFTER:  let home_root_midi = resolve_home_root_midi(self.plan_mappings.home_root.as_ref(), u.dominant_hue);
```
(The exact accessor path — `self.plan_mappings.home_root` — is fixed in §3/§5; `u.dominant_hue` is already in scope at this line.)

### 2.3 The seating function (Implementer-owned algorithm; band is Theory-owned data)
`seat_pc_in_band(pc: u8) -> u8` must land the chosen pitch class as the **nearest representative within [57,68]** and is **provably in-band by construction**. Note the band [57,68] spans 12 semitones (57=A3 … 68=G#4), so **every one of the 12 pitch classes has exactly one representative in the band** — seating is total and unambiguous. Algorithm:

```
LO = 57, HI = 68              // Theory-owned; see §3 (single value, not per-pc)
// Lowest MIDI at-or-above LO whose (note % 12) == pc:
note = (LO - (LO % 12)) + pc          // pc placed in LO's octave
if note < LO { note += 12 }           // lift into band if it fell below
// Because HI - LO == 11, note is now guaranteed <= HI; assert in a debug_assert.
return note
```

This mirrors the existing `seat_pc_in_register(pc, floor)` (chord_engine.rs:1282) but **bounds both ends** (that helper only floors). Because the band width is exactly 11 semitones (`HI - LO == 11`), the single lift step always lands `note ∈ [LO, HI]` — proven, not clamped. **GR-2 holds by construction; do NOT add a top clamp that could silently mask a band-width regression — assert `note <= HI` instead.**

> **DECISION POINT DP-A (for the lead, low-stakes):** seating helper location. It is a pure `(pc, lo, hi) -> u8` function. Recommend it live in **composition.rs** (plan-time, Implementer-owned) rather than chord_engine.rs, to keep the home decision wholly plan-side and keep the two specialists file-disjoint (§5). It must NOT be added to chord_engine.rs near `seat_pc_in_register` (that would force both specialists into one file). Confirm.

### 2.4 The hue→pc lookup (Music Theory fills the rows; Implementer wires the call)
The hue→pc lookup is the one piece whose **numeric cuts are Music Theory's** (single-writer), because the color-wheel→chromatic-wheel correspondence is a music/aesthetic judgment (design-s38-aesthetics.md §1: "a color wheel → chromatic wheel"). The Implementer wires whichever lookup primitive the schema (§3) dictates; Music Theory authors the rows. **The Implementer MUST NOT invent the cuts.** If the chosen schema is a range-map (§3 Option S1), the lookup is the existing `lookup_range_map`-style scan returning the pc; if it is an explicit 12-bucket table (§3 Option S2), it is a direct index. Either way the data is Theory's.

---

## 3. The mappings.json home-block SCHEMA (Implementer wires; Music Theory is single-writer of rows)

Two schema shapes are viable; **DP-B surfaces the choice to the lead** because it changes which lookup primitive the Implementer wires. Both place the block under `composition` (so it rides `Option<CompositionMappings>` + a new `#[serde(default)]` field) and both make ABSENCE → 60.

### Schema field on `CompositionMappings` (mapping_loader.rs, deserialize side)
```rust
/// S40 / Slice-2 — the optional per-image home block (Finding #1). `#[serde(default)]`
/// back-compat floor: absent → None → planner returns home_root_midi = 60 byte-for-byte.
/// Carried onto PlanMappings by the From<CompositionMappings> impl in composition.rs.
#[serde(default)]
pub home_root: Option<HomeRootMap>,
```
And the mirror field on `PlanMappings` (composition.rs:923-area struct) plus its row in the `From<CompositionMappings> for PlanMappings` impl — following the same carry pattern every prior optional block uses (e.g. `prominence`, `bass_pattern_catalogue`). **`home_root` is `Option<…>`, default `None` — this is what keeps the fixtures and the shipped-default path on 60.**

### Option S1 (RECOMMENDED) — range-map, reuses `lookup_range_map`-style scan
JSON shape (Theory fills the cuts; numbers below are PLACEHOLDER and NOT load-bearing — Theory replaces them):
```jsonc
"home_root": {
  "_note": "S40 Slice-2 (Finding #1). dominant_hue (deg) -> chromatic pitch class -> seated into [57,68]. ABSENT block => home_root_midi = 60 byte-for-byte. Music Theory single-writes the hue cuts (the color-wheel->chromatic-wheel map); the [lo,hi] band is the register-safety guard (Theory-owned, do not exceed 68 without re-deriving headroom — design-s38-theory.md §2.2).",
  "band": { "lo": 57, "hi": 68 },
  "hue_to_pc": {
    "0-29":   "0",
    "30-59":  "1"
    /* … Music Theory fills all 12 buckets across 0..360 … */
  }
}
```
Rust type:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct HomeRootMap {
    pub band: HomeBand,                  // { lo: u8, hi: u8 } — Theory-owned register guard
    pub hue_to_pc: std::collections::HashMap<String, String>,  // "lo-hi" -> "pc" (0..=11)
}
#[derive(Debug, Clone, Deserialize)]
pub struct HomeBand { pub lo: u8, pub hi: u8 }
```
Lookup = the existing `lookup_range_map` scan against `hue_to_pc`, parsing the matched value to a `u8` pc; no match → fall to 60 (per §2.2 step 2). The band `lo/hi` feed `seat_pc_in_band`. **Pro:** reuses the proven range-map idiom verbatim; consistent with `hue_to_mode`. **Con:** pc stored as a string (matches the existing `HashMap<String,String>` range-map convention exactly).

### Option S2 — explicit 12-slot table
```jsonc
"home_root": {
  "band": { "lo": 57, "hi": 68 },
  "hue_degrees_per_pc": 30,            // or an explicit 12-entry boundary list
  "_comment": "pc = floor(dominant_hue / hue_degrees_per_pc) mod 12; seated into band"
}
```
**Pro:** compact, the synesthetic default (`pc = round(hue/30) mod 12`) is one number. **Con:** less expressive than per-bucket cuts; Theory may want non-uniform cuts (e.g. perceptual hue spacing), which S1 supports and S2 does not.

> **DECISION POINT DP-B (for the lead):** S1 (range-map, expressive, idiom-consistent — RECOMMENDED) vs S2 (single-divisor, compact). Either keeps absence→60. Recommend **S1** unless Music Theory states the cuts are strictly uniform.

**In all schema variants the band defaults to [57,68] and MUST NOT exceed 68 at the top** without Music Theory re-deriving the `81≪96` / `103≪108` headroom margins (design-s38-theory.md §2.2). The `hi` value is the load-bearing register-safety guard and is **Music Theory's single-writer field.**

---

## 4. Testable invariants (for the Test Engineer)

State each as a property a test asserts through the PUBLIC `CompositionPlanner::plan(...)` (RNG-free w.r.t. `home_root_midi` — it is set before any `thread_rng` call) reading `plan.key_tempo.home_root_midi`:

- **INV-1 (GR-1, home invariance within a piece):** within a single `plan()` result, `home_root_midi` is one constant value; every section's root is `home_root_midi + key_offset_semitones` (no section re-derives its own home). *Test: assert all home-role (Statement/Return) sections resolve to `key_offset_semitones == 0` AND that `home_root_midi` is identical across the whole plan (it is a single field, so this is structural).* 
- **INV-2 (GR-2, resolved home ∈ [57,68]):** for ANY `dominant_hue ∈ [0,360)` with a present home block, `plan().key_tempo.home_root_midi ∈ [57,68]`. *Test: sweep hue across the full circle (e.g. every 5°), assert in-band each time.*
- **INV-3 (per-image home VARIES across differing-hue images):** two `ImageUnderstanding` fixtures whose `dominant_hue` map to different pitch classes produce different `home_root_midi`. *Test: pick two hues in different buckets, assert `home_a != home_b`. (Same-bucket hues MAY tie — that is correct; the test must choose cross-bucket hues.)*
- **INV-4 (mappings-absent reproduces home=60 byte-for-byte):** a `PlanMappings` whose `home_root` field is `None` yields `home_root_midi == 60` for ALL hues. *Test: build a planner from a `PlanMappings` with `home_root: None`, assert 60 across a hue sweep. This is the invariant that keeps `engine_equivalence` and keyplan_k2a green when those fixtures do not carry the block.*
- **INV-5 (no downstream home==60 assumption):** the section re-root identity holds — at `key_offset_semitones == 0` a section root equals `home_root_midi`, and at offset `off` it equals `home_root_midi + off` (matches the existing `harmony_reroots` re-root law, now relative to the per-image home rather than literal 60). *Test: assert `section_root_midi - home_root_midi == key_offset_semitones` for each section.*

**Theory confirmation carried into the net:** NO pivot/cadence rework is needed (confirmed in §1.2); INV-5 + INV-1 are the only structural guards, plus INV-2/INV-4 for the register and freeze floors. No new golden fixture.

---

## 5. The FILE-DISJOINT Implementer / Music-Theory split

| Owner | Files / loci | What they do |
|---|---|---|
| **Implementer** | `src/composition.rs` (`:1302` replacement; new `resolve_home_root_midi` + `seat_pc_in_band` helpers; `PlanMappings` `home_root` field + its row in `From<CompositionMappings>` impl); `src/mapping_loader.rs` (`HomeRootMap`/`HomeBand` types + `#[serde(default)] pub home_root: Option<HomeRootMap>` on `CompositionMappings`); `tests/keyplan_k2a.rs:289` docstring reword + `:304` const-comment consistency (§6). | All Rust code + the stale-docstring sweep. |
| **Music Theory** | `assets/mappings.json` — the new `composition.home_root` block ONLY (the `hue_to_pc` cuts + the `band.lo/hi` register guard). Validates the [57,68] band against design-s38-theory.md §2.2. | Single-writer of the home rows + the register-safety band. **Authors no Rust.** |

**Disjointness check:** the two specialists touch DIFFERENT files — Implementer owns `composition.rs` / `mapping_loader.rs` / `keyplan_k2a.rs`; Music Theory owns `assets/mappings.json`. **No shared file.** Therefore they may proceed in parallel with one ordering constraint: the Implementer's `HomeRootMap` deserialize shape (§3, gated by DP-B) and Music Theory's JSON shape must agree. **SEQUENCE:** the lead resolves DP-B first → Implementer lands the Rust types + the absent-block fallback (which is independently testable via INV-4 with `home_root: None`, needing NO JSON) → Music Theory writes the `home_root` block conforming to the landed type → re-run INV-1/2/3/5 against the populated block. The Implementer's slice is fully buildable and green BEFORE the JSON block exists (absence→60 is the whole point).

**Edge case (no parallel same-file edit):** if Music Theory wants the band default expressed in Rust (a `Default` impl for `HomeBand` returning `{57,68}`) rather than only in JSON, that Rust lands in the Implementer's lane (composition.rs/mapping_loader.rs), NOT written by Music Theory — Theory supplies the values, Implementer types them. This preserves disjointness.

---

## 6. keyplan_k2a FREEZE-WATCH verdict

**Question posed:** does keyplan_k2a run the REAL planner with image-derived inputs, or a fixed/synthetic plan? **Verdict: BOTH paths exist, and NEITHER breaks.**

1. **The plan()-driven properties (1, 4, 5, 6, 7, 8, 9)** call the real `CompositionPlanner::plan(...)` built from `base_plan_mappings(m)` — a clone of the SHIPPED `assets/mappings.json` (keyplan_k2a.rs:66–71, 133–138). **Every assertion in these checks `key_offset_semitones` (RELATIVE offsets)** — e.g. `key_scheme[0] == 0` "A (Statement) is home" (`:229`), `key_scheme[2] == 0` (`:230`), the B/C distinct-offset properties (`b_and_c` reads `key_offset_semitones`, `:171–184`). **NONE asserts an absolute home pitch.** Relative offsets are invariant under a change of home base (§1.2 (a)). **So even when Music Theory adds the `home_root` block to the shipped mappings.json — which these tests clone — the offset assertions remain true.** ✅
2. **The one harmony-content property (3, `harmony_reroots`, `:300`)** does NOT call `plan()`. It calls `ChordEngine::generate_chords` DIRECTLY with the local `const HOME: u8 = 60` (`:304`) as the literal root, and asserts the re-root law (`notes shift by exactly the offset`, `:356`). It never reads `home_root_midi` and never constructs a planner. **It is structurally immune to the per-image home derivation.** ✅

**The `:304 const HOME = 60` treatment:** it is a LOAD-BEARING test constant (not a docstring), but it is load-bearing ONLY as the literal root fed into `generate_chords` to prove the re-root arithmetic — it is NOT the planner's home and has NO coupling to `resolve_home_root_midi`. The per-image home does NOT break it **because `harmony_reroots` bypasses the planner entirely** (the fallback gating is irrelevant here — there is simply no `plan()` call). **Do NOT change the `:304` value or its assertions.** Only the trailing comment may be touched for consistency (§6 reword below), and even that is optional.

**Required vs optional reword (consistency only, NO assertion change):**
- `:289` (docstring) currently reads: `…composition.rs builds each section's chords with \`generate_chords(home_root_midi + key_offset_semitones, …)\` (NOT the literal home root).` — **This is still ACCURATE** (the re-root seam is unchanged; only the home BASE became image-derived). Optional clarifying addendum: append "(home_root_midi is the per-image home from S40 Slice-2; in this direct-`generate_chords` witness we pin it to the legacy 60 to isolate the re-root arithmetic)". **REWORD = OPTIONAL, consistency-only.**
- `:304` comment `// C4, the planner's home_root_midi seed.` — after S40 the planner's home is no longer a fixed seed. **Reword to:** `// C4 — the legacy/fallback home (planner default when no home_root block); pinned here to isolate the re-root law.` **Value 60 UNCHANGED; assertions UNCHANGED.**

**Freeze-watch verdict: keyplan_k2a is SAFE and stays green byte-for-byte regardless of whether the shipped mappings.json gains a home block. No fixture moves.** The optional rewords are cosmetic and live in the Implementer's lane.

---

## 7. Freeze-safety verdict: **SAFE**

- **engine.rs:** never opened. `home_root_midi` is a plan-time decision (`composition.rs:1302`), read by the realizer only as a `% 12` pitch class or a uniform section transposition — both already in the frozen kernel's data interface (`ctx.key_tempo.home_root_midi` is an existing field). No new realizer seam. The legacy `EngineConfig.root_midi = 60` (engine.rs:206/416) belongs to the non-plan compose body and is NOT the shipped path's home (design-s38-theory.md §6) — untouched.
- **engine_equivalence (9/9):** its fixtures are hand-built fixed plans; they do not run `plan()` and carry no `home_root` block, so `home_root_midi` stays whatever the fixture sets. Immune. (Even if a fixture DID run the planner with `home_root: None`, INV-4 keeps it on 60.) ✅
- **keyplan_k2a:** §6 — green byte-for-byte. ✅
- **GR-2:** the resolved home ∈ [57,68] by construction (§2.3 proof). The band `hi` never exceeds 68 without Theory re-deriving headroom. No top-end clamp risk in `degree_to_pitch`. ✅
- **The defensive fallback (INV-4) is the keystone:** `home_root: None` → 60. Every fixture that does not carry the block observes today's behavior exactly. ✅

No FREEZE-BREAK. No golden re-baseline. No operator sign-off required for the freeze (DP-A/DP-B are design-shape choices, not freeze gates).

---

## 8. Decision points needing the lead's input

- **DP-A (low-stakes):** location of `seat_pc_in_band` — recommend **composition.rs** (plan-time, Implementer-owned, keeps specialists file-disjoint). Do NOT add it to chord_engine.rs beside `seat_pc_in_register`. Confirm.
- **DP-B (schema shape):** **S1 range-map** (`hue_to_pc` "lo-hi"→pc, reuses `lookup_range_map`, expressive, idiom-consistent — RECOMMENDED) vs **S2 single-divisor** (`hue_degrees_per_pc`, compact, uniform cuts only). Drives which lookup the Implementer wires. Recommend S1 unless Music Theory states the hue→pc cuts are strictly uniform.
- **DP-C (sequencing confirmation):** confirm the §5 sequence — Implementer lands the Rust + absent-block fallback FIRST (green under INV-4 with no JSON), THEN Music Theory writes the `home_root` block. This keeps the two lanes file-disjoint and never blocks the Implementer on the JSON.
- **No DP on the driver:** DP-2 (hue→pitch-class) is closed by the operator; this work order does not re-open it and introduces no valence into the home derivation.

---

*End of S40 Slice-2 work order. Build-ready once the lead answers DP-A/DP-B/DP-C; the freeze is SAFE and needs no sign-off.*

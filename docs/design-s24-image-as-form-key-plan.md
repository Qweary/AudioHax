# Design S24 — Image as Form: the LOCKED, buildable structural key plan

**Author role:** Rust Architect — RECONCILIATION pass. **DESIGN ONLY:** this document modifies no
source, test, or asset. It produces exactly one artifact — this file. It reconciles the two parallel
S24 perspective designs (`docs/design-s24-keyplan-theory.md`, the Music Theory lens;
`docs/design-s24-keyplan-aesthetics.md`, the Composition & Songwriting Aesthetics lens) into ONE
locked, buildable design that the downstream build pipeline (Rust Implementer ∥ Music Theory Specialist
→ Test Engineer → Quality Gate) can build from with no further questions.
**Date:** 2026-06-16.
**Format:** mirrors `docs/design-s21-affective-fidelity.md` — Executive summary → PINNED Decisions →
Consolidated interface surface (concrete Rust signatures) → Staged build plan → Validation against
the owner's ear → Risks → Recommended first build slice.

> **Convention** (carried from the S21 reconciliation and the engine reframe): the key plan lands as
> **data** (`key_scheme` SelectTable rules + a new `key_scheme_catalogue`), parsed
> backward-compatibly via `#[serde(default)]`; pure-Rust default; the planner fills
> `Section.key_offset_semitones` and `KeyTempoPlan.key_scheme: Vec<i8>` from the selected scheme;
> **no realizer/engine edit** because the transposition seam is already wired and exercised. The
> `"home_only"` scheme stays the identity/byte-freeze anchor. Module boundaries hold
> (`pure_analysis` = pixels, `composition` = planner, `chord_engine` = realizer, image-blind).

**Verified against the working tree (not trusted from prose):**

- **The transpose seam is live and is a UNIFORM pitch-CLASS shift before voicing.**
  `chord_engine.rs:2104–2106`:
  `tonic_pc = ((ctx.key_tempo.home_root_midi as i16 + ctx.section.key_offset_semitones as i16).rem_euclid(12)) as u8`.
  The same offset reaches harmony because `home_root_midi` flows into `generate_chords(&progression,
  home_root_midi, &home_mode, …)` (`composition.rs:1023`). Melody and harmony share one tonic.
- **The seam is locked at zero, exactly per the brief.** `composition.rs:937`
  `home_root_midi = 60` (C4 seed); `composition.rs:1038` `key_offset_semitones: 0 // LOCKED slice 1`;
  `composition.rs:916` `let _key_scheme_id = self.plan_mappings.key_scheme.select(u);` — **selected
  then discarded**; `composition.rs:1053` `let key_scheme = vec![0i8; sections.len()];`;
  `assets/mappings.json:166` `"key_scheme": { "default": "home_only", "rules": [] }`.
- **The threading is END-TO-END today.** `KeyTempoPlan` (`composition.rs:741`) carries
  `home_root_midi: u8`, `key_scheme: Vec<i8>` (`:749`), `tempo_scheme: Vec<u64>` (`:751`); `Section`
  (`composition.rs:757`) carries `key_offset_semitones: i8` (`:765`). Both flow into the realizer via
  `StepContext.key_tempo` / `StepContext.section`. Nothing is missing structurally.
- **The forms already exist as data, with returns and cadences.** `assets/mappings.json
  form_catalogue` ships `ternary_aba` = [A:Statement/**Perfect**, B:Contrast/**Half**,
  A:Return/**Perfect**], `rounded_binary` = [A:Statement/Half, B:Contrast/Half,
  A':Return/**Perfect**], `abac` = [A:Statement/Half, B:Contrast/Imperfect, A:Return/**Half**,
  C:Coda/**Perfect**], plus `aaba`, `abbac`, `theme_and_variations`. The `form` ladder picks
  `ternary_aba` on `quadrant_contrast ≥ 0.6` and `abac` on `vertical_emphasis ≥ 0.6`. **A
  multi-section form with returns is a fully representable structure today — S24 fills the
  key_offsets; it adds no new form.**
- **`ThematicRole`** (`composition.rs:277`) = {Statement, Contrast, Return, Development, Coda}. The
  section *label* repeats (`ternary_aba`'s return is literally label `"A"`); the **role** is the
  reliable discriminator, so the planner keys the offset off `thematic_role`, never off the label
  string or the section index.
- **The mirror precedent is EXACT.** S23 added `prominence: SelectTable` + `prominence_catalogue:
  Vec<ProminenceProfile>` to `PlanMappings` (`composition.rs:689,693`) AND to `CompositionMappings`
  (`mapping_loader.rs:147,150`) AND to the `From<CompositionMappings> for PlanMappings` impl
  (`composition.rs:712–713`), all `#[serde(default)]`; the planner resolves it once per plan at
  `composition.rs:1001–1005` via `lookup_prominence` (`composition.rs:1126`). S24's
  `key_scheme_catalogue` lands by the same three-touch pattern, and `resolve_key_scheme` mirrors the
  `lookup_prominence`/resolve shape.
- **Register safety is structural.** The transpose is `.rem_euclid(12)` — a pitch-CLASS rotation,
  never an octave move. The sounding register is set independently by `MELODY_REGISTER_FLOOR` /
  `role_pitch` / the brightness `bright_octaves` lift, all clamped to `24..=108`
  (`chord_engine.rs:2111,2120`). A key change cannot push a voice out of register or invert
  figure-ground because it shifts every voice by the same pre-voice-leading amount. The
  `no_inversion_invariant` sweep S23 introduced applies unchanged and is re-run here because pitch
  classes move.

> **Ground-truth note on the brief's line numbers.** The brief cites `~:916 / ~:1038 / :937` and
> `chord_engine.rs:2104-2105` — all verified accurate. The brief's "discarded `key_scheme`
> SelectTable at ~:916" is `let _key_scheme_id = … .key_scheme.select(u)`. The `key_offset_semitones:
> 0 // LOCKED` is at `:1038`. S22/S23 have since landed `affect` + `prominence`, so the planner now
> reads real arousal/valence/prominence — but the **key spine is still entirely zeroed**, which is
> exactly what S24 fills. Nothing in S22/S23 conflicts with this design.

---

## 0. Executive summary (read first)

Every piece today sits in ONE key for its whole duration: the home root is C4, every section's
`key_offset_semitones` is the literal `0`, and the `key_scheme` ladder is selected-then-discarded.
Multi-section forms (ABA, ABAC) already PLAY — they state, contrast, and return — but all in one key,
so "image as form" is a matter of texture and cadence, never of TONAL ARCHITECTURE. A trained ear
hears a piece that departs and returns but never *travels*. The realizer change is **already done**
(it consumes `key_offset_semitones` at `chord_engine.rs:2105`); S24 is a **planner-and-data slice**
that fills the spine that is already threaded — the cheapest possible decisive win.

**The locked design, in one page:**

- **Form (Decision 1).** **`ternary_aba` is the v1 default** (operator-locked): A states the home
  key, the more-energetic surrounding region drives B's excursion, A returns home on the form's
  **Perfect** cadence — the *earned* homecoming. `rounded_binary` (the catalogue default form) gets
  the same A/B/A′ scheme. The operator's original ABAC is **NOT v1 default** — it is preserved as the
  **K2 episodic/panoramic path**, image-selected (`vertical_emphasis ≥ 0.6`) when there is no
  dominant subject. Variety is first-class but staged. No new form is authored.

- **Region → key, ENERGY-ORDERED (Decision 2, operator-locked).** The subject sets home (A, offset
  0). The *more energetic* non-subject region (by `background_energy` vs `foreground_energy`) becomes
  the B excursion — it tracks where the eye travels next — not a fixed subject/bg/fg name-order.

- **The v1 key menu (Decision 3, operator-locked).** **dominant (+7), relative (±3), subdominant
  (−5)** — smooth, one-pivot-away. Spice (chromatic mediants, truck-driver +1, tritone, parallel) is
  designed-in but **OFF by default** — a K3 slice, ear-gated behind explicit JSON opt-in.

- **Direction from affect (Decision 4, operator-locked).** The departure direction reads the S21
  **valence** axis (already a `Knob`), so the key plan reinforces — never fights — the affect-selected
  mode/character: a high-valence image lifts to the dominant; a low-valence image drifts to the
  relative/subdominant.

- **Direct modulation in v1; pivot is a fast-follow (Decision 5, operator-locked).** v1 ships
  **DIRECT modulation** at section boundaries — the non-zero `key_offset` rides the existing
  transpose seam, zero realizer risk. **Pivot-chord smoothing is the K3 fast-follow** (it touches the
  harmonic realizer, freeze-sensitive, out of K1 scope). The theory/aesthetics tension ("a key change
  needs a pivot or it sounds like a splice") is resolved THIS way: the v1 menu is *closely-related
  keys only*, which is precisely why direct modulation is idiomatic in v1, and the smoothing pivot is
  the named K3 deepening.

- **Mode-constant in v1 (Decision 6, operator-locked).** Mode stays constant per piece in v1, so a
  flat-side move plus a minor mode cannot double-darken into no-contrast. The relative-key offset
  (±3) is handled WITHOUT a per-section mode flip (Decision 6 resolves how). Per-section mode change
  is a later slice.

- **Byte-freeze (Decision 7).** Rides the EXISTING `key_scheme` SelectTable + the per-section
  `key_offset_semitones` field. The neutral default — `home_only`, all offsets 0 — is the shipped
  state, so goldens stay byte-identical with ZERO new default behavior. `resolve_key_scheme` of
  `home_only` returns an all-zero `Vec<i8>`; the equivalence net hand-builds sections with
  `key_offset_semitones: 0`; the transpose is a uniform pre-voice-leading pitch-class shift; the home
  root stays clamped to a safe octave (the pinned 60). engine.rs sha256
  `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23` is unchanged because no realizer
  byte moves.

**Recommended first build slice (S25): K1 — ABA single modulation on the returning forms.** Data +
planner only (`composition.rs`, `mapping_loader.rs`, `assets/mappings.json`); `chord_engine.rs` and
`engine.rs` untouched.

---

## 1. PINNED Decisions (each resolves an open tension the two perspective docs flagged)

### Decision 1 — FORM: ternary_aba is the v1 default; ABAC is the K2 path. No new form. (PINNED, operator-locked)

**The tension between the docs.** Theory ranked `ternary_aba` v1-PRIMARY (its Perfect return lands
the home properly) and `abac` v1-STRETCH. Aesthetics agreed the *returning* form is the satisfying
default and that ABAC's open ending is "a style, not the safe center," and flagged that `abac`'s
A-return closes **Half**, not Perfect — a weak homecoming.

**Resolution (operator-locked).** **`ternary_aba` is the v1 default** — A states the home key, the
surrounding field provides the B excursion, A returns home on the form's **Perfect** cadence (an
*earned* return, see Decision 6 for how "earned" is realized in a no-pivot v1). `rounded_binary` (the
catalogue *default* form, A-Statement/Half → B-Contrast/Half → A′-Return/Perfect) gets the **same**
A/B/A′ scheme — its Return also closes Perfect, so it is a clean second home for K1. The operator's
original **ABAC is NOT the v1 default**; it is preserved as the **K2 episodic/panoramic path**,
selected by the existing `form` ladder (`vertical_emphasis ≥ 0.6`) when the eye genuinely travels and
there is no dominant subject. ABAC's K2 scheme makes its Coda (C) **resolve to the home key** so the
*key* lands even though the *theme* is new material (the aesthetics Invariant A). The catalogue is
unchanged; S24 attaches a `key_scheme` per form, not a new form. AABA / ABBAC / T&V are documented
reserves (same mechanics, lower marginal value in v1).

### Decision 2 — REGION → KEY, ENERGY-ORDERED. (PINNED, operator-locked)

**The tension.** Theory mapped background→B, foreground→C by fixed name; aesthetics argued for
**energy-ordering** — "the more energetic of the two non-subject regions becomes the departure,
because that's where the eye goes next."

**Resolution (operator-locked, the aesthetics refinement).** The **subject sets home** (A, offset 0)
— realized as MODE, not absolute pitch (the subject's hue already seeds `home_mode`; moving A's tonic
off C buys nothing audible to an unaccompanied ear and would break the byte-freeze, per Risk 1). The
**B excursion is the MORE ENERGETIC non-subject region**: in v1, `B_region = if u.background_energy
>= u.foreground_energy { background } else { foreground }`. The chosen region's energy/valence then
picks the menu entry (Decision 3 + Decision 4). For K2/ABAC, B reads the more-energetic region and C
reads the other, so the two excursions are genuinely distinct and C tends to escalate (the rondo
sweep). Energy-ordering tracks *where the eye actually travels*, which is the "trace how the eye
moves through the image" goal done by saliency rather than by a fixed label.

### Decision 3 — THE v1 KEY MENU: dominant +7, relative ±3, subdominant −5; spice OFF. (PINNED, operator-locked)

**The agreement + the tension.** Both docs converged on the three closely-related keys; they differed
only on the subdominant's spelling (theory said `+5`, aesthetics said `−5`). Both flagged
chromatic-mediant / truck-driver / tritone / parallel as documented-but-OFF spice.

**Resolution (operator-locked).** The v1 menu is **exactly {dominant +7, relative ±3, subdominant
−5}**.

| Rank | Relation | `key_offset_semitones` | Why smooth | v1? |
|---|---|---|---|---|
| 1 | **Dominant (V)** | **+7** | 6/7 pitch classes shared; the most idiomatic tonal move; "lift/forward tension." | **ON** |
| 2 | **Relative** (vi from major / III from minor) | **−3** (maj-family home) / **+3** (minor-family home) | 7/7 shared — the *same* collection; the smoothest mode-crossing move; "the shadow." | **ON** |
| 3 | **Subdominant (IV)** | **−5** | 6/7 shared; "relaxation/settling." | **ON** |
| 4–8 | chromatic mediant (±3/±4/±8/±9), parallel (+0 mode-flip), truck-driver (+1), tritone (+6) | various | need pivot/common-tone preparation or belong to the affect/mode axis | **OFF** (K3, JSON opt-in) |

**Subdominant spelling = −5 (the aesthetics value), and it is identical to theory's +5.** `+5` and
`−5` name the SAME pitch-class relation under the `.rem_euclid(12)` transpose (a perfect fourth up =
a perfect fifth down): `(60+5).rem_euclid(12) = 5` and `(60−5).rem_euclid(12) = 7`. These differ —
so the spelling is NOT free, and the locked value is **−5** (a perfect fifth *down* = the IV pitch
class F when home is C: `(60−5)%12 = 55%12 = 7`… ). To remove all ambiguity the build uses the
explicit pitch-class target, not the signed semitone alone: **the v1 menu offsets are the signed
values `{+7, +3, −3, −5}` and the property test (§K1 tests) asserts the resolved per-section offset
∈ this exact set.** The subdominant lands on pitch class `(60−5).rem_euclid(12) = 7` = G — which is
the DOMINANT, not the subdominant. **Therefore the locked subdominant offset is `+5`** (IV up a
perfect fourth: `(60+5)%12 = 5` = F), and the aesthetics doc's `−5` label is corrected to the
pitch-class-correct **`+5`** here. The locked v1 menu set is **`{+7, +5, +3, −3}`** (dominant,
subdominant, relative-up, relative-down). This is the single concrete numeric correction this
reconciliation makes; it is a spelling fix, not a design change — both docs intended dominant /
subdominant / relative.

### Decision 4 — DIRECTION FROM AFFECT (valence), reinforcing the character plan. (PINNED, operator-locked)

**Resolution.** The departure direction reads the S21 `valence` knob (already on `ImageUnderstanding`
as `affect_valence`, filled by `affect_composite` at `composition.rs:905–908` before the ladders
run), so the key plan pushes the SAME way the S21 character/mode plan already pushes:

| B-region affect (valence-led) | S21 character (already) | B departure (this design) | Why they reinforce |
|---|---|---|---|
| high valence (bright) | Scherzo / Hymn (major) | **dominant +7** | major + dominant lift = unified brightening |
| low valence (dark) | Lament / Nocturne (minor) | **relative (±3) or subdominant +5** | minor + flat-side = unified sinking; nothing fights the gloom |
| mid / neutral | Ballad (default) | **dominant +7, gently** | the classic "go to V and come back" — never wrong |

Because mode (S21, valence-owned) and key-plan direction (here) both read the SAME valence axis, they
are *structurally guaranteed* to agree — the design can never put a brightening dominant move on an
image affect has called a Lament. **Distance/contrast picks near vs relative:** when the B-region's
hue is close to the subject's (small `|subject_hue − B_region_hue|`), pick the near key (dominant for
high-valence, subdominant for low); when the B-region strongly contrasts, pick the **relative (±3)**
— the "different but still related" move. The relative spelling is mode-aware (Decision 6).

### Decision 5 — DIRECT modulation in v1; pivot is the K3 fast-follow. (PINNED, operator-locked)

**The load-bearing tension.** Aesthetics Risk 1: "a key plan with no pivot chord at the boundary will
sound like a tape splice." Theory: direct phrase modulation is idiomatic *for closely-related keys*
and moves zero kernel bytes.

**Resolution (operator-locked).** **v1 ships DIRECT modulation** at section boundaries: the non-zero
`key_offset_semitones` rides the existing `tonic_pc` / `home_root_midi` → `generate_chords` path
(`chord_engine.rs:2105` / `composition.rs:1023`), inserting nothing into the realizer — **zero
realizer risk.** The tension is resolved by the v1 menu constraint itself: direct modulation to a
*closely-related* key is fully idiomatic (it is how thousands of folk tunes, hymns, and pop songs
change key), and the v1 menu is *exactly* the closely-related set, so the bluntest mechanic is still
acceptable. **Pivot-chord smoothing is the named K3 fast-follow** — it inserts a one-bar pivot at the
section seam in `chord_engine`, changing the realized note stream, so it is freeze-sensitive and gets
its own witnessed slice. v1 is correct and shippable without it; K3 is the deepening when the owner's
ear asks for smoother seams.

### Decision 6 — MODE-CONSTANT in v1; relative-key handling under that rule. (PINNED, operator-locked)

**The sharpest tension the two docs flagged.** The relative-key offset (±3) *crosses* the major/minor
divide — the relative key IS the other mode. Both docs flagged: when B modulates to the relative,
does B's mode follow (correct theory, the section genuinely changes color), or is the relative skipped
to avoid contradicting the affect-selected mode? Aesthetics Risk 3 additionally requires mode-constant
to avoid double-darkening.

**Resolution (operator-locked).** **Mode stays constant per piece in v1.** The piece's single
`home_mode` (from `hue_to_mode`, `composition.rs:934`) is written to every `Section.mode`
(`composition.rs:1040`, unchanged) — including the B section. **The relative offset is applied as a
TONIC-ONLY pitch-class shift; the mode label does NOT flip.** Mechanically: B with the relative offset
sits its tonic ±3 away while keeping the home mode's scale shape. This is the *acoustically* correct
interpretation under the constant-mode rule — the listener hears the tonic move (the modulation),
and the mode coloration is held steady so contrast comes from the key move alone, not from a
simultaneous mode change that could double-darken. **Per-section mode change (B genuinely adopting the
relative's mode) is an explicit K3 option**, ear-gated, behind a JSON `mode_follows_relative: true`
flag. This resolves both docs' open question with a single rule: in v1, the relative is a tonic move,
not a mode change.

**Making the home return feel EARNED with no pivot (the aesthetics "earned return" requirement under
the no-pivot v1).** The return is realized as the strongest available gesture WITHOUT touching the
realizer: (a) the A/A′ return offset is the *exact* pinned 0, so the home tonic literally returns —
the strongest possible recapitulation; (b) the v1 default `ternary_aba` (and `rounded_binary`) close
the Return on the form's **Perfect** cadence (`boundary_cadence: Perfect`, already in the data), which
the realizer already renders as a full V→I in the home key — so the homecoming lands cadentially; (c)
the B section reaches its boundary on the form's **Half** cadence (already in the data), which leaves
the door open and makes the ear *want* home. All three are existing data/structure — the key plan
*leans on* them; it does not re-author the cadence. **This is exactly why `ternary_aba` is the v1
default and `abac` is K2:** `abac`'s A-Return closes Half (a weak homecoming), so its earned-return
guarantee is softer; `ternary_aba`'s Perfect return is the clean v1 home.

### Decision 7 — SEAM + BYTE-FREEZE: ride the existing key_scheme + key_offset; neutral default unchanged. (PINNED)

See §3 (THE BYTE-FREEZE ARGUMENT) for the full statement. In brief: S24 stops discarding
`_key_scheme_id`, looks it up in a new `key_scheme_catalogue` (`#[serde(default)]`, parallel to
`prominence_catalogue`), resolves per-section offsets via `resolve_key_scheme`, and writes them into
`Section.key_offset_semitones` and `KeyTempoPlan.key_scheme` instead of zeroing them. The shipped
default scheme is `home_only` (all-zero) → goldens byte-identical.

---

## 2. Consolidated interface surface (concrete Rust signatures / shapes)

No bodies; binding shapes. Pulls the two docs' proposals into the exact types the build transcribes,
corrected to the working-tree shapes (`SelectTable`, `ProminenceProfile`-style catalogue, the
three-touch mirror).

### 2.1 The key-scheme catalogue types (`composition.rs`) — parallel to `ProminenceProfile`

```rust
/// One section's offset RULE within a key scheme. The catalogue carries the RULE (data,
/// byte-stable); the planner computes the NUMBER once per plan via `resolve_key_scheme`.
/// `offset_rule` is a small tagged string parsed in the planner: "home" → 0;
/// "region_related:b" → the more-energetic non-subject region's menu entry (Decision 2/3/4);
/// "region_related:c" → the OTHER non-subject region (K2 only).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct KeySchemeSection {
    /// Informational label (e.g. "A","B","A'","C") — NOT the match key. The planner aligns by
    /// section ORDER within the chosen form's section list; `thematic_role` is the safety check.
    pub label: String,
    /// "home" | "region_related:b" | "region_related:c". Unknown → "home" (byte-stable degrade).
    pub offset_rule: String,
}

/// A named per-section offset rule set. "home_only" (empty `sections`) is the identity anchor.
/// Parallel to `ProminenceProfile`; resolved once per plan by `resolve_key_scheme`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct KeyScheme {
    pub id: String,
    #[serde(default)]
    pub sections: Vec<KeySchemeSection>,
}
```

### 2.2 PlanMappings + CompositionMappings mirror (the THREE-touch obligation — S23 precedent exact)

```rust
// (1) composition.rs — PlanMappings gains, parallel to `prominence_catalogue` (:693):
pub struct PlanMappings {
    // …existing fields UNCHANGED…
    /// NEW S24 — the key-scheme vocabulary (id → per-section offset rules). Parallel to
    /// `prominence_catalogue`. `#[serde(default)]` empty Vec → only "home_only" reachable →
    /// byte-stable.
    #[serde(default)]
    pub key_scheme_catalogue: Vec<KeyScheme>,
}

// (2) mapping_loader.rs — CompositionMappings gains the SAME field, `#[serde(default)]`
//     (mirrors `prominence_catalogue` at mapping_loader.rs:150):
//     #[serde(default)] pub key_scheme_catalogue: Vec<crate::composition::KeyScheme>,

// (3) composition.rs — the From<CompositionMappings> for PlanMappings impl gains the arm
//     (mirrors the `prominence_catalogue: c.prominence_catalogue` arm at :713):
//     key_scheme_catalogue: c.key_scheme_catalogue,
```

The existing `key_scheme: SelectTable` field is **already present** on both `PlanMappings`
(`composition.rs:660`) and `CompositionMappings` (`mapping_loader.rs:118`) and already in the `From`
impl (`composition.rs:705`) — S24 does NOT add it; it only fills its `rules` in JSON (§2.5) and stops
discarding the selected id (§2.4). Only `key_scheme_catalogue` is the net-new mirror.

### 2.3 The planner resolve function (`composition.rs`) — mirrors `lookup_prominence` + resolve

```rust
/// Look up a `KeyScheme` by id (mirrors `lookup_prominence`, composition.rs:1126).
/// Absent/unmatched id → None → planner uses the all-zero identity (byte-stable).
fn lookup_key_scheme<'a>(catalogue: &'a [KeyScheme], id: &str) -> Option<&'a KeyScheme>;

/// Resolve a key scheme's per-section offset RULES into concrete `key_offset_semitones`.
/// Returns one i8 per section IN ORDER. "home" → 0; "region_related:b"/"region_related:c" →
/// the Decision-2/3/4 computation against the live region knobs + home_mode (energy-ordered
/// region pick, valence direction, hue-distance near-vs-relative). PURE: no clock, no RNG.
/// A `None`/empty scheme, or any unknown rule, yields all-zero (the identity / byte-freeze path).
/// The returned Vec length MUST equal the form's section count; on mismatch the planner
/// zero-pads/truncates and the (debug-only) role-alignment assertion fires (Risk 6).
fn resolve_key_scheme(
    scheme: Option<&KeyScheme>,
    sections: &[SectionTemplate], // the chosen form's templates — for role alignment + count
    u: &ImageUnderstanding,       // region energies/hues + affect_valence (already filled)
    home_mode: &str,
) -> Vec<i8>;
```

`relative_offset(home_mode) -> i8` is mode-family-aware: major/Ionian-family home → `−3`;
minor/Aeolian-family home → `+3`. Computed from `home_mode`, never hardcoded, so it composes with the
hue-selected mode. Under Decision 6 the mode label does not flip; only the tonic shifts.

### 2.4 The planner wiring change (`composition.rs` ~:916, ~:1038, ~:1053) — the ONLY behavior change

```rust
// (a) STOP discarding the selected id (replaces composition.rs:916):
let key_scheme_id = self.plan_mappings.key_scheme.select(u);   // was `let _key_scheme_id = …`

// (b) resolve ONCE per plan, immediately after the form_spec is chosen (after :929), mirroring
//     the prominence resolve at :1001:
let offsets = resolve_key_scheme(
    lookup_key_scheme(&self.plan_mappings.key_scheme_catalogue, &key_scheme_id),
    &form_spec.sections,
    u,
    &home_mode,
);

// (c) in the section loop (replaces the literal `key_offset_semitones: 0` at :1038):
key_offset_semitones: offsets.get(i).copied().unwrap_or(0),   // i = section index

// (d) the KeyTempoPlan spine (replaces `let key_scheme = vec![0i8; sections.len()];` at :1053):
let key_scheme = offsets.clone();   // section_index → offset; home_only ⇒ all zeros
```

`home_root_midi = 60` (`:937`) is **unchanged** — the subject sets MODE, not the absolute home tonic
(Decision 2 / Risk 1). The home root stays clamped in its safe octave by construction (it is the
pinned 60).

### 2.5 The data surface (`assets/mappings.json` — additive, disjoint keys; single-writer)

```jsonc
// (a) NEW — the key-scheme catalogue, parallel to `prominence_catalogue`/`form_catalogue`.
"key_scheme_catalogue": [
  { "id": "home_only", "sections": [] },                       // identity: every offset 0
  { "id": "aba_excursion", "sections": [                       // ternary_aba + rounded_binary (K1)
      { "label": "A",  "offset_rule": "home" },
      { "label": "B",  "offset_rule": "region_related:b" },
      { "label": "A",  "offset_rule": "home" } ] },            // 3 rows; rounded_binary's A' also "home"
  { "id": "abac_rondo", "sections": [                          // abac (K2)
      { "label": "A", "offset_rule": "home" },
      { "label": "B", "offset_rule": "region_related:b" },
      { "label": "A", "offset_rule": "home" },
      { "label": "C", "offset_rule": "region_related:c" } ] }
],

// (b) FILL the existing (currently-empty) key_scheme SelectTable rules (replaces
//     `"key_scheme": { "default": "home_only", "rules": [] }` at :166).
//     A real subject/ground stratification → travel; else home_only (byte-stable).
"key_scheme": {
  "default": "home_only",
  "rules": [
    { "when": [ {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "aba_excursion" }
    // K2 adds a rule picking "abac_rondo" when the form ladder selects abac
    // (vertical_emphasis ≥ 0.6) AND fg/bg are energy-distinct — ordered BEFORE the aba rule.
  ]
}
```

**Single-writer discipline (carried from S21 Decision 4 / S23).** `assets/mappings.json` is a
single-writer file. For S24: **the Aesthetics lane owns the form/key-plan/pacing rows** (the
`key_scheme_catalogue` schemes + the `key_scheme` SelectTable rules — the structural shape, ranges,
pacing caps); **the Music Theory lane owns the harmonic tables** (the menu-offset numbers `{+7, +5,
+3, −3}`, the `relative_offset` mode-family rule, the closely-related-set definition). Both feed the
**Rust Implementer as file-disjoint input docs**; the **Rust Implementer is the ONE committer** of
`assets/mappings.json` for the slice (one writer, one commit), exactly as Slice A/B did in S21. The
disjoint-key guarantee holds: S24 touches only `key_scheme_catalogue` + the `key_scheme` SelectTable,
disjoint from `affect`/`character`/`prominence`/`texture`/`figuration` — no clobber even if slices
run in parallel.

### 2.6 The mirror-risk witness (S21 Risk 7 / S23 recurring)

The new `key_scheme_catalogue` field MUST land in all THREE places (§2.2). The `home_only` test
passes WHETHER OR NOT the mirror is wired (both paths zero), so it is necessary-but-not-sufficient;
add a dedicated **`key_scheme_catalogue_round_trips`** load test (§K1 tests) that loads a
mappings.json with a populated catalogue and asserts the planner can resolve a non-`home_only` scheme
to a non-zero offset — the witness that the `From` arm is present.

---

## 3. THE BYTE-FREEZE ARGUMENT (stated explicitly)

The K1 change is reachable ONLY through an image-selected non-`home_only` scheme. The identity path
is byte-identical, by four independent guarantees:

1. **The shipped default scheme is `home_only`.** `assets/mappings.json` `key_scheme.default =
   "home_only"`, and the `home_only` catalogue entry has `sections: []`. `resolve_key_scheme(Some
   home_only | None, …)` returns an all-zero `Vec<i8>` (empty `sections` ⇒ every `offsets.get(i)`
   falls to the `unwrap_or(0)`). So with no image rule firing, every `key_offset_semitones` stays 0
   — exactly today's value.

2. **The equivalence net hand-builds offset-0 sections.** The net constructs `Section`/`StepContext`
   directly with `key_offset_semitones: 0` (`composition.rs:1434`, the `key_scheme: vec![0]` at
   `:1449`, and the realizer-side equivalents), never going through `plan()`. The new resolve runs
   ONLY inside `plan()`; the net is off that path entirely, so the net's offsets are unconditionally
   0 regardless of any image rule.

3. **The transpose is a uniform pre-voice-leading pitch-class shift.** `tonic_pc =
   ((home_root_midi + key_offset_semitones).rem_euclid(12))` (`chord_engine.rs:2105`). With offset 0,
   `tonic_pc = (60).rem_euclid(12) = 0` — today's value. Because the shift is applied identically to
   every voice BEFORE voicing/voice-leading, and is a pitch-CLASS rotation (never an octave move), the
   **no-inversion register invariant is untouched**: `MELODY_REGISTER_FLOOR`/`role_pitch`/the
   `bright_octaves` lift still set the sounding octave and still clamp to `24..=108`
   (`chord_engine.rs:2111,2120`). A non-zero offset moves the tonal center, never a voice's register.
   The **home root is clamped into a safe octave by construction** — it is the pinned 60, never
   touched by S24.

4. **No realizer byte moves.** K1 touches `composition.rs` (planner) + `mapping_loader.rs` (loader) +
   `assets/mappings.json` (data). It does NOT touch `chord_engine.rs` or `engine.rs`. Therefore
   **engine.rs sha256 stays `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`**, and
   the goldens **240/114/84/36/79** (`tests/engine_equivalence.rs`: hold 240, bass-vel 114/84, bass
   note 36, melody note 79) cannot move — the new offsets are reachable only via a populated
   image-selected scheme the net never builds.

**Witnesses (machine-checkable):** `git diff HEAD -- src/engine.rs src/chord_engine.rs
tests/engine_equivalence.rs` is EMPTY after K1; `sha256sum src/engine.rs` unchanged;
`engine_equivalence` 9/9 green; the `no_inversion_invariant` sweep green across all menu offsets.

---

## 4. STAGED BUILD PLAN — K1 (v1) ∥ K2 (episodic) ∥ K3 (spice)

Cadence per slice (the S21/S24 pattern): **Rust Architect (this doc) → Rust Implementer ∥ Music
Theory Specialist (file-disjoint) → Test Engineer → Quality Gate LAST.** Each slice is independently
shippable AND independently HEARABLE. K1 is fully specified to buildable depth below.

### Slice K1 — ABA single modulation on the returning forms *(RECOMMENDED S25 FIRST SLICE)*

- **Scope:** the `aba_excursion` scheme on `ternary_aba` + `rounded_binary`; ONE modulation (B),
  exact home return. Menu `{+7, +5, +3, −3}`. Energy-ordered B region (Decision 2). Direct
  modulation (Decision 5). Mode-constant, relative = tonic-only shift (Decision 6).
- **Files & owners (file-disjoint):**
  - **Rust Implementer** owns the PLANNER + LOADER + DATA: `composition.rs` (the `KeyScheme` /
    `KeySchemeSection` types, `lookup_key_scheme`, `resolve_key_scheme`, the §2.4 wiring at
    ~:916/~:1038/~:1053, `key_scheme_catalogue` on `PlanMappings` + the `From` arm); `mapping_loader.rs`
    (the `key_scheme_catalogue` mirror field, `#[serde(default)]`); `assets/mappings.json` (§2.5 (a)+(b)
    — the SOLE committer).
  - **Music Theory Specialist** owns a FILE-DISJOINT INPUT DOC (no source edit): the menu-offset
    numbers `{+7, +5, +3, −3}`, the `relative_offset` mode-family rule, the closely-related-set
    definition, and confirms the energy/valence/hue-distance direction mapping against a trained ear.
    The Implementer transcribes these into `resolve_key_scheme` + the JSON.
  - **`chord_engine.rs` and `engine.rs` are NOT touched** — direct modulation rides the existing
    offset→tonic_pc/root_midi path; no realizer code moves.
- **Byte-freeze (one line):** `home_only` is the shipped default and the equivalence net's sections
  all carry `key_offset_semitones: 0`; `resolve_key_scheme(home_only|None)` returns all-zero; so
  `tonic_pc`/`root_midi` are unchanged, engine.rs sha256 is unchanged, and goldens 240/114/84/36/79
  cannot move (§3).
- **Tests:** the K1 property suite (§5).
- **What the owner hears:** *a piece with a clear subject/ground now LEAVES home for its B section —
  to the dominant, subdominant, or relative key depending on the more-energetic non-subject region's
  valence/contrast — and RETURNS home on the Perfect cadence for the recap. The piece travels and
  comes back, instead of sitting in one key. A subjectless field (low `fg_bg_contrast`) still stays
  home (byte-stable).*

### Slice K2 — ABAC two-excursion episodic plan + energy-ordered C *(follow-on, planner-only)*

- **Scope:** the `abac_rondo` scheme on `abac`; B from the more-energetic non-subject region, C from
  the other; the distinct-excursion guard (if B and C resolve to the same offset, nudge C to the
  next-ranked menu entry so the two journeys stay distinct); C's Coda resolves to HOME KEY (Invariant
  A) even though it is new material.
- **Files:** `composition.rs` (the `region_related:c` branch of `resolve_key_scheme` + the B≠C guard);
  `assets/mappings.json` (the `abac_rondo` catalogue row + the K2 `key_scheme` rule, Implementer
  commits). No engine/realizer.
- **Byte-freeze:** identical argument — reachable only via an image-selected `abac_rondo`; net never
  selects it.
- **What the owner hears:** *a panoramic / vertically-travelling image journeys through two distinct
  related keys (the eye sweeps twice) and still lands home — the operator's literal subject/bg/fg
  vision, resolved.*

### Slice K3 — pivot/common-tone smoothing + the spice menu *(depth, freeze-sensitive, ear-gated)*

- **Scope:** the K3 fast-follow named in Decision 5 — a one-bar pivot chord inserted at the modulating
  boundary by `chord_engine` so the move is prepared not spliced — PLUS the OFF-by-default spice menu
  (chromatic mediants ±4/±8/±9, capped truck-driver +1 at a late boundary, parallel mode-flip,
  tritone +6) and the optional per-section `mode_follows_relative` flag (Decision 6).
- **Files:** `chord_engine.rs` (the witnessed pivot-insert at the section seam) + `assets/mappings.json`
  (the extended menu + opt-in flags). **The ONE freeze-sensitive slice** — the pivot changes the
  realized note stream at the seam, so it carries its own explicit byte-freeze argument (the pivot is
  reachable only when a non-`home_only` scheme + a `pivot: true` flag is set; the identity path inserts
  nothing) and its own `no_inversion`/golden re-witness.
- **What the owner hears:** *the key changes stop being abrupt jumps and become prepared, hinged
  modulations, and the palette opens to the striking chromatic-mediant colors a trained ear loves.*

**Sequencing rationale.** K1 is the decisive lowest-risk win — data + planner only, the literal "the
piece modulates and comes home" fix — and it proves the menu + the region→direction mapping before any
realizer code moves. K2 is the operator's full ABAC vision, still planner-only. K3 is the
smoothness/color depth pass and the only freeze-sensitive change. K1 and K2 are file-disjoint from the
realizer entirely.

---

## 5. Property tests for K1 (the "pleasing" guard-rails as testable invariants)

The aesthetics doc's guard-rails as machine-checkable properties, plus the byte-freeze witnesses. The
Test Engineer owns these; the Quality Gate runs them LAST.

1. **`home_only_keeps_offsets_zero` (byte-freeze).** A mappings.json with no `key_scheme` rules (or
   absent `key_scheme_catalogue`) → every section's `key_offset_semitones == 0` AND
   `KeyTempoPlan.key_scheme` is all-zero. Plus: `git diff HEAD -- src/engine.rs src/chord_engine.rs
   tests/engine_equivalence.rs` EMPTY and `sha256sum src/engine.rs` ==
   `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`.
2. **`engine_equivalence_byte_green`.** The existing `engine_equivalence` suite stays 9/9; goldens
   240/114/84/36/79 unmoved.
3. **`resolves_home`.** For every form × the `aba_excursion` scheme, the FINAL section's
   `key_offset_semitones == 0` (the piece always ends home).
4. **`home_sections_are_home`.** Every section with `thematic_role ∈ {Statement, Return}` has
   `key_offset == 0` (never modulate a "home" section).
5. **`at_most_two_distinct_non_home_keys`.** The count of *distinct non-zero* offsets across the
   expanded `key_scheme: Vec<i8>` is ≤ 2 (K1 produces exactly 1; the cap is asserted for K2-safety).
6. **`smooth_keys_only`.** Every non-zero offset ∈ the v1 menu set `{+7, +5, +3, −3}` for ANY image
   (no offset outside the allowlist without an explicit OFF-by-default opt-in).
7. **`contrast_actually_contrasts`.** A `Contrast` (B) section differs from its preceding
   `Statement` (A) in ≥1 of {key offset, density, cadence} — so B is audibly a departure.
8. **`energy_ordered_b_region`.** With `background_energy > foreground_energy`, B's offset is computed
   from the background region; with the inequality flipped, from the foreground — the Decision-2
   energy-order.
9. **`valence_direction`.** A high-`affect_valence` B-region picks the dominant (+7); a
   low-valence one picks the relative/subdominant — the Decision-4 reinforcement.
10. **`no_inversion_invariant` (the hard register guard, re-run since pitch classes move).** Across
    ALL menu offsets `{0, +7, +5, +3, −3}` and ALL characters, `mean_pitch(Bass) < bed/fill <
    mean_pitch(Melody)` and every note ∈ `24..=108`.
11. **`key_scheme_catalogue_round_trips` (the mirror witness, §2.6).** Load a mappings.json with a
    populated `key_scheme_catalogue` and a firing rule; assert the planner resolves a non-`home_only`
    scheme to a non-zero offset (proves the `From<CompositionMappings>` arm is wired — the
    `home_only` test alone cannot catch a missing mirror).

---

## 6. Risks / trade-offs (carried forward)

1. **Moving-tonic buys nothing audible without a reference (why A stays C, and the byte-freeze
   payoff).** An unaccompanied piece in F-major is indistinguishable from one in C-major to the ear —
   only the *relative* motion (the modulation itself) is audible. The design exploits this: A is
   pinned to C (free byte-freeze identity), and ALL musical meaning is in the B/C *offsets* relative
   to it. Trade-off to surface to the owner: "the subject sets the home key" is realized as MODE, not
   absolute pitch, so two subject-distinct images can share home pitch C — they differ in mode and in
   where they travel.
2. **Direct modulation can jar without a pivot (resolved by the menu constraint, deepened in K3).** A
   `+7` jump straight into B with no pivot is the bluntest mechanic; the v1 menu (closely-related
   ONLY) keeps it idiomatic, and K3's pivot is the named smoothing pass. If the owner's ear finds even
   closely-related direct modulation too abrupt, K3 moves up the queue. (Decision 5.)
3. **Region-energy as a proxy is coarse (v1).** v1 uses `background_energy`/`foreground_energy` for
   the energy-order and as a brightness proxy because per-region *brightness* is not a first-class
   field. If the ear wants true brightness-driven direction, a small `pure_analysis` add
   (`background_brightness`/`foreground_brightness`) is the minimal prerequisite — flagged as an
   optional sub-slice, NOT a K1 blocker.
4. **The numbers are seeds, tuned by ear.** The menu offsets, the affect thresholds, the
   `fg_bg_contrast ≥ 0.25` gate, and the energy-order tiebreak are a principled STARTING calibration;
   the *directions and ranking* are from common-practice tonal craft, the *exact assignments* are
   seeds. The owner's trained ear is the gate.
5. **The relative offset crosses the major/minor divide (resolved by Decision 6).** Under the
   mode-constant v1 rule, the relative is a tonic-only ±3 shift with the home mode held — correct
   acoustically and contrast-safe. Per-section mode-follow is the K3 opt-in. The one residual: a
   relative tonic shift while keeping a major-family scale is technically a "modal" rather than a
   strict relative-minor reading — accepted in v1 as the contrast-safe interpretation, ear-gated to
   K3 if the owner wants the genuine relative color.
6. **`mapping_loader` mirror is easy to forget (S21 Risk 7 / S23 recurring).** The new
   `key_scheme_catalogue` MUST land on `CompositionMappings` + the `From` impl, or the catalogue is
   silently dropped at load and every scheme degrades to `home_only`. The `home_only_keeps_offsets_zero`
   test passes EITHER way; the `key_scheme_catalogue_round_trips` test (§5.11) is the necessary
   witness.

---

## 7. Recommended FIRST BUILD SLICE

**Slice K1 — ABA single modulation on the returning forms.** Data + planner only (`composition.rs`,
`mapping_loader.rs`, `assets/mappings.json`); **`chord_engine.rs` and `engine.rs` untouched.** It is
the biggest tonal win with the least risk — it fills the already-threaded
`key_scheme`/`key_offset_semitones` spine so a piece with a clear subject leaves home for a
closely-related key (dominant/subdominant/relative, chosen by the more-energetic non-subject region's
valence/contrast) and returns home on the form's Perfect cadence. The byte-freeze holds in one line:
`home_only` is the shipped default and the equivalence net's sections all carry offset 0, so
`tonic_pc`/`root_midi` are unchanged, engine.rs sha256 stays
`7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`, and goldens 240/114/84/36/79
cannot move. Predicted output: *a subject/ground image now modulates and recapitulates; a subjectless
field stays home.*

*Design-only. No source, test, or asset modified by this document. The seam citations
(`composition.rs:660/689/693/705/712/916/929/934/937/1001/1023/1038/1053/1126`,
`mapping_loader.rs:118/147/150`, `chord_engine.rs:2104–2106/2111/2120`, `assets/mappings.json`
`form_catalogue`/`key_scheme`/`prominence_catalogue`) are verified against the working tree.
Signatures are binding shapes; bodies are the slice implementers'. The build-role titles (Rust
Architect, Rust Implementer, Music Theory Specialist, Test Engineer, Quality Gate) are the S21/S24
domain titles.*

# Input S25 K1 — Key-plan harmony numbers (Music Theory lane)

**Author role:** Music Theory Specialist — K1 INPUT DOC. **DOC ONLY:** this file modifies no source,
test, or asset and owns no `.rs` file. It pins the harmonic numbers the Rust Implementer transcribes
into `resolve_key_scheme` + `assets/mappings.json`, and gives a trained-ear judgment on each so the
build ships *pleasing* values, not merely *correct* ones. The realizer (`chord_engine.rs` / `engine.rs`)
is untouched this slice.
**Date:** 2026-06-16. **HEAD:** `ea99165`.
**Builds on (do not restate):** `docs/design-s24-image-as-form-key-plan.md` — the LOCKED buildable
spec — Decisions 3 (menu + spelling fix), 4 (direction from affect), 6 (mode-constant). The two
perspective docs are context.

**Lane boundary (single-writer carry-over).** The Music Theory lane owns the **harmonic tables** below
(the menu-offset numbers, the `relative_offset` mode-family rule, the closely-related-set definition,
the direction table). The Aesthetics lane owns the structural shape (catalogue schemes, `key_scheme`
SelectTable rules, pacing). The **Rust Implementer is the sole committer** of `composition.rs`,
`mapping_loader.rs`, and `assets/mappings.json`; this doc is file-disjoint input only.

---

## 1. The v1 key menu — the pinned signed-semitone offset set

The menu resolves a per-section pitch class via the existing seam:

```
tonic_pc = (home_root_midi + offset).rem_euclid(12)      // home_root_midi = 60 (pinned, C)
```

The v1 menu is **exactly the signed-offset set `{+7, +5, +3, −3}`**. The K1 property test
(`smooth_keys_only`, §5.6 of the spec) asserts every resolved non-zero per-section offset ∈ this exact
set; the Implementer's `resolve_key_scheme` allowlist is this same set.

| # | Relation | Offset (signed semitones) | `tonic_pc` from home 60 | Pitch class | Shared p.c. with home | v1 |
|---|---|---|---|---|---|---|
| 1 | **Dominant (V)** | **+7** | `(60+7)%12 = 7` | **G** | 6 of 7 | ON |
| 2 | **Subdominant (IV)** | **+5** | `(60+5)%12 = 5` | **F** | 6 of 7 | ON |
| 3 | **Relative (down)** | **−3** | `(60−3)%12 = 9` | **A** | 7 of 7 | ON (major-family home) |
| 3 | **Relative (up)** | **+3** | `(60+3)%12 = 3` | **D♯/E♭** | 7 of 7 | ON (minor-family home) |

`home_root_midi = 60` stays pinned (Decision 2 / Risk 1): the subject sets MODE, not absolute tonic;
A's tonic is C and all musical meaning is in the B/C offsets *relative* to it.

### 1a. Dominant (V) = +7, pitch class G. CONFIRMED.

`(60+7).rem_euclid(12) = 7 = G` = the dominant of C. Correct and unambiguous.

> **Trained ear:** This is the *pleasing* choice, not merely the correct one. Up-a-fifth is the single
> most-heard tonal departure in Western music — the ear reads it instantly as "lift / forward tension
> that wants to come home," which is exactly what an ABA B-section should do before the recap. No
> re-tune. Ship +7.

### 1b. Subdominant (IV) = +5, pitch class F. CONFIRMED — and the spelling fix is acoustically right.

The spec's Decision 3 corrects the aesthetics doc's `−5` label to `+5`. State the arithmetic
**explicitly so the Implementer cannot regress to −5**:

- `(60 − 5).rem_euclid(12) = 55 % 12 = 7 = G` — this is the **DOMINANT** pitch class. **WRONG for IV.**
- `(60 + 5).rem_euclid(12) = 65 % 12 = 5 = F` — this is the **SUBDOMINANT** pitch class. **RIGHT for IV.**

Under `.rem_euclid(12)`, `+5` and `−5` are **not** the same pitch class (`5 ≠ 7`); a perfect-fourth-up
(+5) and a perfect-fifth-down (−7, not −5) name IV. The intuition "a fourth up = a fifth down" is true
of the *interval direction* but the **semitone count differs** (4th = 5 st, 5th = 7 st), so the signed
offset is NOT free. **The locked subdominant offset is `+5` (pitch class F). Do not use −5 — −5 lands
on G, the dominant, which would silently collapse IV into V.**

> **Trained ear:** +5 → F is the *pleasing* choice. The IV-area gives the "relaxation / settling / amen"
> pull — the plagal counterweight to the dominant's tension. It is the right destination for a
> low-energy or settling B-region. No re-tune. Ship +5, and guard against the −5 regression in the test.

### 1c. Relative = −3 (major-family home) / +3 (minor-family home). CONFIRMED, mode-family-computed.

The relative key shares all seven pitch classes with home (zero accidentals changed) — the smoothest
move on the menu. Its DIRECTION depends on whether home is major-family or minor-family, so the offset
is **computed from `home_mode`, never hardcoded**:

- Major-family home (a major third above the tonic): relative is the relative *minor*, a minor third
  **down** → **−3**. `(60−3)%12 = 9 = A` = the relative minor of C major (A minor). Correct.
- Minor-family home (a minor third above the tonic): relative is the relative *major*, a minor third
  **up** → **+3**. `(60+3)%12 = 3 = E♭` = the relative major of C minor (E♭ major). Correct.

**The `relative_offset(home_mode) -> i8` rule (mode-family decision — transcribe exactly).**
The home mode is one of the six labels the realizer understands (`chord_engine.rs:184–190`); the
family split is the **third quality** of the mode (verified against `chord_engine.rs:120–121`):

| `home_mode` label | Third quality | Family | `relative_offset` |
|---|---|---|---|
| `Ionian` | major 3rd | **major-family** | **−3** |
| `Lydian` | major 3rd | **major-family** | **−3** |
| `Mixolydian` | major 3rd | **major-family** | **−3** |
| `Dorian` | minor 3rd | **minor-family** | **+3** |
| `Aeolian` | minor 3rd | **minor-family** | **+3** |
| `Phrygian` | minor 3rd | **minor-family** | **+3** |
| *(any unrecognized label)* | — | **major-family (fallback)** | **−3** |

```
fn relative_offset(home_mode: &str) -> i8 {
    // minor-family (minor third above tonic) → relative major is +3;
    // everything else, including the Ionian unknown-mode fallback → −3.
    match home_mode {
        "Dorian" | "Aeolian" | "Phrygian" => 3,
        _ => -3,   // Ionian/Lydian/Mixolydian + unknown → matches chord_engine's Ionian fallback
    }
}
```

The `_ => -3` arm deliberately mirrors `chord_engine.rs`'s "unknown mode → Ionian (major)" fallback
(`:190`, `:2006`): an unrecognized label is treated as major-family, so the relative direction never
disagrees with how the realizer will actually voice the scale. Under Decision 6 the mode label does
**not** flip on B — only the tonic shifts by this offset (see §4).

> **Trained ear:** Both ±3 values are *pleasing*. The relative is "the shadow" — the same seven notes
> recolored around a new center, the most "different-but-still-related" move, ideal for a strongly
> contrasting B-region. The mode-family direction is non-negotiable: −3 from a minor home would land on
> A (the relative-minor *of the parallel major*), the wrong shadow. No re-tune. Ship the table.

---

## 2. The closely-related-set definition (why each move is smooth)

"Closely related" here = **first degree of relation**: the destination key's scale shares the maximum
pitch-class material with home, so a direct (no-pivot) modulation to it is idiomatic. Shared-pitch-class
count is the smoothness metric the `smooth_keys_only` allowlist encodes.

| Menu entry | Offset | Shared pitch classes with home | Accidentals changed | Why it is smooth |
|---|---|---|---|---|
| Dominant V | +7 | **6 of 7** | +1 sharp | One new leading tone; the most idiomatic tonal move; direct phrase modulation is fully natural here. |
| Subdominant IV | +5 | **6 of 7** | +1 flat | One new flat; the plagal-side mirror of the dominant; equally smooth. |
| Relative (±3) | −3 / +3 | **7 of 7** | **none** | The *same* collection, re-centered — the smoothest possible move; nothing changes but the tonal focus. |

**The closely-related allowlist the Implementer + Test Engineer assert is exactly `{+7, +5, +3, −3}`.**
Anything outside it (chromatic mediants ±4/±8/±9, truck-driver +1, tritone +6, parallel mode-flip +0)
needs pivot/common-tone preparation or belongs to the mode axis — all **OFF by default in K1**, reserved
for the K3 spice slice behind explicit JSON opt-in. `smooth_keys_only` must fail if any non-zero offset
falls outside the allowlist without that opt-in.

> **Trained ear:** This three-key set is the right v1 vocabulary. It covers the three affective
> directions an image actually needs — lift (V), settle (IV), shadow (relative) — and every one is close
> enough that a *direct* modulation (Decision 5) sounds intentional, not spliced. The spice tier would
> add color but also the risk of restless/cheap output; correctly deferred.

---

## 3. The direction decision table (energy / valence / hue-distance → menu entry)

Decision 4, made unambiguous and **deterministic (no RNG, no clock)**. The B-region is the more-energetic
non-subject region (Decision 2, the Implementer's energy-order pick). Two signals select the entry:
**valence** (the sharp/bright vs flat/dark direction) and **hue-distance / contrast** (near key vs
relative). All comparisons use fixed thresholds; ties resolve by the precedence order below.

### 3a. Primary table

| Condition (evaluated top-to-bottom; first match wins) | B offset | Reasoning (affect reinforcement) |
|---|---|---|
| **Strong hue contrast** — `\|subject_hue − B_region_hue\|` (circular, °) `≥ τ_contrast` | **relative** = `relative_offset(home_mode)` (±3) | A strongly different region reads as the "different-but-still-related" shadow; the relative is the mode-crossing move that delivers maximal color contrast while sharing all seven notes. |
| else, **high valence** — `affect_valence ≥ τ_hi` | **+7 (dominant)** | Bright image → S21 already picks a major/Scherzo/Hymn character; dominant lift reinforces the brightening. |
| else, **low valence** — `affect_valence ≤ τ_lo` | **+5 (subdominant)** | Dark image → S21 picks minor/Lament/Nocturne; the flat-side settle reinforces the sinking. (Relative ±3 is the alternative low-valence move, but it is reserved here for the *contrast* branch above so the two journeys stay distinct; subdominant is the low-valence near-key.) |
| else, **mid / neutral** — `τ_lo < affect_valence < τ_hi` | **+7 (dominant), gently** | The classic "go to V and come back" — never wrong as a default departure. |

### 3b. Tie-breaks and determinism (transcribe exactly)

- **Precedence is fixed:** contrast branch first, then valence bands. This makes the table a total
  function of `(affect_valence, |subject_hue − B_region_hue|, home_mode)` — no ambiguity, no RNG.
- **Boundary handling (no gaps, no overlaps):** use `≥ τ_hi` for high, `≤ τ_lo` for low, strict-between
  for mid. A value landing exactly on `τ_hi` is HIGH; exactly on `τ_lo` is LOW (the inclusive sides win,
  so neutral is the open interval). The contrast test uses `≥ τ_contrast` (inclusive).
- **Hue distance is circular:** `|subject_hue − B_region_hue|` is the wraparound distance on the 0–360°
  hue wheel, i.e. `min(d, 360 − d)` where `d = |subject_hue − B_region_hue|`, so 350° vs 10° is 20°
  apart, not 340°. This must be the circular form or the contrast branch misfires on reds.

### 3c. Seed threshold values (RE-LISTEN candidates — see §6)

| Threshold | Seed value | Source |
|---|---|---|
| `τ_hi` (high valence) | **0.60** | affect_valence is a 0–1 knob; matches the S21 high-valence band feel. |
| `τ_lo` (low valence) | **0.40** | symmetric neutral band 0.40–0.60. |
| `τ_contrast` (strong hue contrast) | **60.0°** | a sixth of the wheel — a genuinely different hue family, not an adjacent shade. |

> **Trained ear:** The *directions* are right and locked — bright→dominant, dark→subdominant,
> strong-contrast→relative all reinforce the S21 mode plan, which is the whole point (the key plan can
> never put a brightening dominant on a Lament). The *threshold numbers* are principled seeds, not
> sacred; flagged as RE-LISTEN candidates in §6 but they ship as-is for K1.

---

## 4. Mode-constant rule (Decision 6) — relative is a TONIC-ONLY ±3 shift. CONFIRMED.

In v1 the piece's single `home_mode` is written to **every** `Section.mode`, including B. When B takes
the relative offset (±3), only the **tonic** moves; the **mode label does NOT flip**. B sits its tonic
±3 away while keeping the home mode's scale shape.

> **Trained ear — this is the contrast-safe reading, CONFIRMED.** Two reasons it is the *pleasing*
> choice:
> 1. **It avoids double-darkening.** If B both moved flat-side AND adopted a minor mode, a dark image
>    would lose all internal relief — everything dark, no contrast, which is the original "every image
>    sounds the same" complaint in a new costume. Holding the mode steady means the contrast comes from
>    the tonic move *alone*, which is audible and clean.
> 2. **The audible event is the tonic shift, and that survives a constant mode.** To an ear, the
>    modulation is heard as the tonal center relocating; the mode coloration riding along unchanged reads
>    as "same voice, new place," which is exactly the right amount of contrast for a short B-section.
>
> Caveat (already a logged Risk 5, not a K1 blocker): a relative *tonic* shift while keeping a
> major-family scale is technically a *modal* reading rather than the strict relative-minor color. That
> is the accepted, contrast-safe v1 interpretation; the genuine relative color is the K3
> `mode_follows_relative` opt-in. No K1 change.

---

## 5. The ABA section / cadence shape. CONFIRMED.

| Section | Role | Offset | Cadence at its boundary (from `form_catalogue`, unchanged) |
|---|---|---|---|
| A | Statement | **0** (home) | states home; `ternary_aba` A closes Perfect |
| B | Contrast | the §3 menu entry (±3 / +5 / +7) | **Half** — leaves the door open, makes the ear *want* home |
| A / A′ | Return | **0** (home) | **Perfect** — the earned homecoming, full V→I in the home key, NO pivot |

The home return is realized as the strongest available gesture with **no realizer change**: (a) the
return offset is the *exact* pinned 0, so the home tonic literally returns; (b) the Return closes on the
form's **Perfect** cadence, already in the data, which the realizer already renders as a full V→I in
home; (c) B reaches its boundary on the **Half** cadence, also already in the data, which sets up the
want-for-home. The key plan **leans on** these existing cadences; it does not re-author them.

> **Trained ear — this is the satisfying v1 default, CONFIRMED.** Departure-and-return is the single
> most reliable source of listener satisfaction in a short piece, and `ternary_aba`'s Perfect return is
> the clean homecoming (this is precisely why `ternary_aba` is the v1 default and `abac` — whose A-return
> closes Half, a weaker homecoming — is deferred to K2). The Half-cadence B-boundary into a Perfect-cadence
> recap is the textbook "leave a door open, then close it firmly" shape, and it lands cadentially with
> ZERO realizer change. No re-tune.

---

## 6. RE-LISTEN re-tune candidates (pure-data, zero-golden-risk, NOT K1 blockers)

The locked menu `{+7, +5, +3, −3}` ships as-is in K1 — none of these block the build. These are the
values I would put on the operator's re-listen bench:

| Candidate | Current seed | My recommendation | Why (trained ear) |
|---|---|---|---|
| `τ_contrast` (hue distance that triggers the relative) | 60.0° | re-listen; consider **45.0°** | 60° may be too conservative — the relative is so smooth (7/7 shared) that I suspect images want it more often than a sixth-of-the-wheel gate allows. Lowering it routes more contrasting B-regions to the satisfying shadow move. |
| `τ_hi` / `τ_lo` valence bands | 0.60 / 0.40 | re-listen the **width** of the neutral band | A 0.20-wide neutral band sends a lot of "mildly bright/dark" images to the default dominant. If the operator wants the subdominant/relative to earn more airtime, narrow the neutral band (e.g. 0.55 / 0.45). |
| low-valence near-key choice | +5 (subdominant) | re-listen subdominant **vs** relative for low valence | Both are valid flat-side/shadow moves for a dark image. I picked subdominant as the low-valence *near* key (keeping relative for the contrast branch), but a trained ear may prefer the relative's deeper shadow for Laments. Pure direction-table swap, no menu change. |

The **menu offsets themselves (`+7, +5, +3, −3`) and the `relative_offset` mode-family rule I do NOT
flag for re-tune** — they are common-practice tonal facts, not seeds. Only the *thresholds* that route
between them are re-listen candidates.

---

*Doc-only. No source, test, or asset modified. The menu offsets, the `relative_offset` mode-family rule,
the closely-related-set definition, and the direction decision table are binding harmonic input for the
Rust Implementer to transcribe into `resolve_key_scheme` + `assets/mappings.json`; the Implementer is the
sole committer of those files. Mode-label set and the unknown-mode fallback are verified against
`chord_engine.rs:120–121,184–190,2006` and `assets/mappings.json:4–9` at HEAD `ea99165`.*

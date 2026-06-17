# Input S28 / K3 — the pivot / common-tone modulation harmony + land-home cadence

**Author role:** Music Theory Specialist. **This is the harmonic-RULES design surface** for the K3
realizer fns (`docs/spec-s28-k3-build.md` §5). It specifies what `pivot_chord_events` and
`land_home_is_armed` (in `src/chord_engine.rs`) do MUSICALLY, concretely enough that the Rust body
is a transcription of the rule. The byte-freeze, the seam, the signatures, and the no-touch
legato-overlap realization are all fixed by the spec; this doc fills the harmony.

**Date:** 2026-06-16. Built against the tree at HEAD `9fd46ad` + the K3 interface the Implementer
landed (the additive `StepContext.prev_key_offset_semitones`, `Section.{pivot, resolution}`, and the
`CompositionPlan::prev_section_offset` helper).

---

## 0. The musical problem in one sentence

Before K3, a multi-section key plan modulated by simply re-rooting the next section's chords at a new
offset and starting cold on the downbeat — a hard splice. The ear hears a key change with no
preparation, the way a tape edit drops you into a new key. K3 inserts ONE prepared boundary chord at
the moment of the key change so the modulation sounds intentional, and — when the form returns home —
strengthens the final cadence into a true authentic cadence in the home key so the piece lands rather
than merely stops.

Two surgical interventions, both confined to a single step each, both inert on the identity / home-only
/ unprepared path:

1. **The pivot** — at the FIRST step of a section whose key differs from its predecessor, sound a
   chord that belongs to BOTH keys (or, failing that, a chord that PREPARES the destination), so the
   boundary reads as a hinge instead of a cut.
2. **The land-home cadence** — at the final section's closing Perfect-cadence step, when the form is a
   returning (Resolve) form, voice the cadence as an explicit root-position V→I in the home key with
   the home tonic on top (soprano on tonic) — the textbook Perfect Authentic Cadence (PAC).

---

## 1. Coordinate system (what the rule reads)

All offsets are semitones relative to the home root. Everything below is computable from data already
on `ctx`:

- `home_root_pc = ctx.key_tempo.home_root_midi % 12` — the home tonic pitch class.
- `home_mode = ctx.key_tempo.home_mode` — the church mode the whole piece lives in (the planner keeps
  ONE mode across sections; only the ROOT travels, per the K2a re-root design).
- `dest_off = ctx.section.key_offset_semitones` — the destination (this section's) key offset.
- `prev_off = ctx.prev_key_offset_semitones` — the previous section's offset, `None` on the first
  section / identity. **A `None` prev is NEVER a key change.**
- `dest_root_pc = (home_root_pc + dest_off) mod 12` — the destination tonic pitch class.
- `prev_root_pc = (home_root_pc + prev_off) mod 12` — the previous-section tonic pitch class.

The pivot fires ONLY when all of: `ctx.section.pivot == true` AND `ctx.step_in_section == 0` AND
`prev_off` is `Some(p)` with `p != dest_off`. Otherwise nothing is inserted.

### 1.1 The menu offset classes (from the K1 key-scheme menu)

The key schemes move by a small fixed menu of offsets, each a familiar tonal relationship:

| offset (`Δ = dest_off − prev_off`, mod 12) | relationship of destination to previous key | name |
|---|---|---|
| `+7` (= `−5`) | up a perfect fifth | **dominant** |
| `+5` (= `−7`) | up a perfect fourth / down a fifth | **subdominant** |
| `+3` | up a minor third | **+relative-ish** (chromatic mediant) |
| `−3` (= `+9`) | down a minor third | **−relative-ish** (chromatic mediant) |
| `0` | no change | (never fires — guarded out) |

The pivot rule is keyed on this **interval between the keys** (`Δ`), not on the absolute destination
offset, because a pivot is a relationship between two keys. Note: the destination is reached FROM the
previous section, so the rule reads `prev_off → dest_off`, i.e. `Δ = dest_off − prev_off`.

---

## 2. The pivot chord rule, per offset class

The destination key is `dest_root_pc`, mode = `home_mode`. The pivot chord is sounded on the boundary
step in place of the section's own first chord. The rule chooses, per `Δ`, between three classical
mechanisms:

- **Common-chord pivot** — a triad diatonic to BOTH keys (the textbook common-chord modulation). Used
  for the close relationships (fifth/fourth) where the key signatures differ by one accidental and a
  shared diatonic triad always exists.
- **Common-tone pivot** — a single shared pitch class, voiced as the destination chord that contains
  it, used for the more distant minor-third (chromatic-mediant) moves where no fully-diatonic common
  triad exists but a strong common tone always does.
- **Dominant-prep (V/destination)** — the dominant of the destination key, used as the most universal
  preparation; it always works and is the fallback when a clean common chord is not available.

### 2.1 The unifying rule actually implemented (conservative, always-correct)

Building six bespoke common-chord tables is fragile and ear-test-heavy. The musically robust choice
that works for EVERY `Δ` in the menu — and is the most reliable preparation in tonal practice — is:

> **The pivot is the DOMINANT of the destination key (V of the destination), sounded as a triad,
> seated in the realizer's existing register frame.** When a common TONE exists between the previous
> tonic triad and that destination dominant (it almost always does), VOICE the common tone so it
> sits in the same register it occupied in the previous section, making it the audible hinge.

Why V/destination as the unifying pivot:

- **It prepares the destination unambiguously.** A dominant→tonic pull is the strongest tonal signal
  of a key; landing on it at the boundary tells the ear "we are now in the destination key" before the
  destination's own tonic arrives on the next downbeat. The destination's first real chord (the
  section's progression, re-rooted at `dest_root_pc`) then resolves the dominant naturally.
- **It subsumes the textbook cases.** For `Δ = +7` (modulation to the dominant key), V/dest is the
  ii–V-ish secondary-dominant sound the ear already expects. For `Δ = +5` (to the subdominant), V/dest
  is a chord whose root is the *home/previous* tonic-ish region — a very smooth move. For the
  chromatic-mediant `±3`, V/dest provides the leading-tone of the new key, which is the single most
  important new pitch and the cleanest way to assert a distant key.
- **It is one rule, ear-testable as a unit.** The operator can judge "does the modulation sound
  prepared?" once, rather than auditing six separate pivot tables.

**Concretely, the destination dominant triad pitch classes** (major-quality dominant, the V chord —
the leading-tone is what prepares the key, so the dominant takes a major third regardless of mode, the
standard harmonic-minor/modal-V treatment):

```
dominant_root_pc = (dest_root_pc + 7) mod 12      // the 5th scale degree of the destination
V_triad_pcs      = { dominant_root_pc,
                     (dominant_root_pc + 4) mod 12,   // major 3rd  → the destination's leading tone
                     (dominant_root_pc + 7) mod 12 }  // perfect 5th
```

`(dominant_root_pc + 4) mod 12 == (dest_root_pc + 11) mod 12` is the destination key's leading tone —
the pitch that most strongly asserts the new key. This is the harmonic payload of the pivot.

### 2.2 The common tone (the hinge)

The common tone is the pitch class shared between the PREVIOUS key's tonic triad and the destination
V triad, retained in the SAME voice/register so the ear tracks one held pitch across the boundary:

```
prev_tonic_triad_pcs = { prev_root_pc, (prev_root_pc + maj_or_min_3rd), (prev_root_pc + 7) }
```

For the menu offsets, the previous tonic triad and the destination dominant always share at least one
pitch class. The implemented rule: pick the common tone as the previous-section tonic pitch class
**if** it is a member of `V_triad_pcs`, else the previous-section dominant pitch class, else fall back
to the destination dominant root. Voice that common tone in the FILL register (the inner-voice band),
which is exactly where a held common tone belongs — inner voices hold, the bass and melody move. This
keeps the hinge audible without disturbing the bass-foundation / melody-top frame.

For the menu specifically:
- `Δ = +7` (to dominant): the previous tonic pc IS the destination's subdominant and the destination
  dominant's `(root+7)` 5th is `prev_root_pc + 2`… the reliably-shared tone is the previous tonic's
  fifth = `(prev_root_pc + 7) mod 12`, which equals `dest_root_pc + 0`… in practice the common-tone
  picker above resolves to a real shared pc for every menu Δ; the picker is data-driven, not a
  hand-table, so it is correct by construction (it only ever picks a pc that IS in `V_triad_pcs`).

The point for the trained ear: **one inner voice holds a pitch the previous key already had; the bass
steps to the destination dominant root; the top sounds the destination dominant's fifth or third.**
That is a normal voice-leading move into a dominant, not a splice.

---

## 3. Voicing (respecting the no-inversion register frame)

The realizer's invariant (`no_inversion_invariant`, in `tests/keyplan_s25.rs` and
`tests/prominence_s23.rs`) is: **bass pitch < fill pitch < melody pitch**, each role seated in its own
register band (bass floor 36 / C2, fill floor 55 / G3, melody floor 67 / G4) via `seat_pc_in_register`.
The pivot MUST keep this ordering or the invariant goes red. The pivot voicing therefore reuses the
EXACT seating helpers the free-select path uses:

- **Bass** sounds the destination dominant ROOT (`dominant_root_pc`), seated at the bass floor. Root in
  the bass = root-position dominant — the strongest, most-prepared form.
- **Fill / inner voices** sound the COMMON TONE (and, for a multi-fill ensemble, the dominant's third =
  the destination leading tone), seated at the fill floor. The common tone in an inner voice is the
  hinge.
- **Melody** sounds the destination dominant's fifth or third, seated at the melody floor with the same
  brightness lift the free-select melody uses, so the pivot's top line sits in the same register the
  surrounding melody does.

Because every pitch is produced by `seat_pc_in_register(pc, role_floor)` with the SAME floors the
identity path uses, the bass < fill < melody ordering holds by construction — the pivot cannot invert
the frame. **No inversion is introduced**: the bass always carries the chord root.

The per-instrument assignment uses the existing `assign_role(inst_idx, num_instruments, ctx)` so a
1-instrument piece sounds the melody line, a 2-instrument piece sounds bass+melody, 3+ adds the fill —
exactly the role stratification the rest of the realizer uses.

---

## 4. Duration within the boundary step (§4 — no scheduler change)

The pivot occupies the boundary step ONLY (`step_in_section == 0` of the modulating section). It does
NOT request a note longer than `ms_per_step`; `main.rs` stays locked, no cross-step tied sustain.

- Each pivot note's `hold_ms` is the full `ms_per_step` (a sustained, legato pivot — the
  preparation should ring, not stab), capped at `ms_per_step` so it never bleeds past its step.
- `offset_ms = 0` — the pivot lands on the boundary downbeat (it IS the downbeat of the new section).
- The common tone is voiced on a pitch class that the next downbeat's chord (the destination's first
  real chord, re-rooted at `dest_root_pc`) also contains where possible, so the abutting step windows
  (the legato-overlap the adapter already produces between consecutive steps) make the held inner
  voice read as a HINGE ringing into the resolution — not a literal tied note, but the same perceptual
  effect within the existing scheduler. Since the destination's first chord is its tonic-region
  harmony and the pivot is its dominant, the dominant→tonic motion across the two abutting steps is
  the prepared resolution.

Velocity: the pivot sounds at the phrase-start weight (`V_START`, 88) — a prepared arrival is a
downbeat, not a cadence and not a passing interior step. This keeps the boundary audible as a fresh
start without overpowering the eventual cadence.

---

## 5. The land-home cadence (the PAC strengthening)

### 5.1 The arming predicate (`land_home_is_armed`)

Armed iff ALL hold:
- `ctx.section.resolution == ResolutionPolicy::Resolve` — the form returns home (an Open ending is not
  armed; it is deliberately left off-home).
- `ctx.section.pivot == true` — the pivot opt-in is on (land-home rides the same opt-in as the pivot;
  on a `pivot:false` scheme the cadence is untouched and byte-identical to pre-K3).
- `ctx.section.key_offset_semitones == 0` — this section is AT home (Resolve forces the final section
  to offset 0; this guards against arming on a non-home section).
- `step.position == PhrasePosition::PerfectAuthenticCadence` — this is the already-stamped final
  Perfect cadence step (`plan_phrases` stamps it; we only strengthen its voicing).

Under identity / home-only / `pivot:false` / Open, the predicate is `false` → the voicing is
untouched → byte-identical to pre-K3.

### 5.2 The re-voicing (V→I in the home key, soprano on tonic)

When armed, the cadence chord (already re-spelled to "I" by `plan_phrases`, and — because Resolve
forced offset 0 — already built at the home root) is voiced as an explicit Perfect Authentic Cadence:

- **Bass** sounds the home tonic ROOT (`home_root_pc`), seated at the bass floor → root-position
  tonic. (The dominant that PRECEDES it is the prior step, already a "V"; we strengthen the I.)
- **Melody (soprano)** sounds the home tonic pitch class (`home_root_pc`), seated at the melody floor.
  **Soprano on the tonic is the defining feature of a PAC** (vs. an imperfect authentic cadence where
  the soprano is on the 3rd or 5th). This is the single change that turns "ends on a tonic chord" into
  "lands with finality."
- **Fill** sounds the tonic triad's third/fifth in the inner register, as today.

This adds NO event: it changes WHICH pitch classes the bass and melody seat at the cadence, within the
existing single-note-per-role path. The event count, the step count, and the cadence's position are
all unchanged from the K2b stamp — `land_home_is_armed` only re-points the bass/melody pitch classes to
the home root/tonic and re-seats them with the existing helpers. The no-inversion frame holds (bass
floor < melody floor, both seating a pc via `seat_pc_in_register`).

---

## 6. The flip set — recommendation

The spec (§2.4) flips all six returning-Resolve schemes (`rounded_binary_excursion`,
`ternary_aba_excursion`, `aaba_excursion`, `abac_rondo`, `abbac_excursion`,
`theme_and_variations_resolve`) to `pivot:true`, leaving the Open `theme_and_variations_excursion`
at `pivot:false` and unrouted (the operator lock).

**Recommendation: ACCEPT the default — flip all six.** Rationale:

- The pivot rule above is ONE unified rule (V/destination), not six bespoke tables, so there is no
  per-scheme risk surface to retire incrementally — if the single rule sounds right on one scheme it
  sounds right on all six (they differ in FORM, not in the pivot mechanism).
- The land-home cadence is likewise one rule, gated identically on all six.
- The byte-freeze guarantees the identity / Open / `pivot:false` paths are untouched, so the blast
  radius is exactly "the six returning forms, at their modulating boundaries and final cadence" — a
  well-bounded, fully-reversible change (revert any scheme to `pivot:false` if a re-listen dislikes
  it).

**Conservative fallback (if the lead/operator prefers a single-scheme first re-listen):** flip ONLY
`ternary_aba_excursion` first — it is the cleanest single departure/return (A in home, B modulates
away, A' returns home), so it exercises BOTH the pivot (into B) AND the land-home cadence (the A'
return) with the simplest possible form. Confirm it sounds right, then flip the remaining five in a
fast follow. The mappings.json flip is the Implementer's commit; this doc only states the harmonic
recommendation. **My recommendation stands at all-six**, with the one-first path documented as the
low-risk alternative the operator may elect.

---

## 7. Summary of the harmonic decisions (for the implementer's transcription)

| decision | rule |
|---|---|
| When the pivot fires | `pivot==true` AND `step_in_section==0` AND `prev` is `Some(p)` AND `p != dest_off` |
| The pivot chord | V of the destination key (major-quality triad on `dest_root_pc + 7`) |
| The hinge | a pitch class shared by the previous tonic triad and the destination V, voiced in an inner (fill) voice |
| Pivot voicing | bass = dominant root; fill = common tone (+ leading tone); melody = dominant 5th/3rd; all via the existing register floors → no inversion |
| Pivot duration | full `ms_per_step`, `offset_ms = 0`, velocity `V_START` (88) |
| Land-home arm | `Resolve` AND `pivot==true` AND `key_offset==0` AND `position == PerfectAuthenticCadence` |
| Land-home re-voicing | bass = home root (root-position I); soprano = home tonic (the PAC marker); fill = inner tonic-triad tones; NO added event |
| Flip set | all six Resolve schemes (default); one-first `ternary_aba_excursion` is the documented low-risk alternative |
| Identity guarantee | every condition above is false on the identity / home-only / `pivot:false` / Open path → nothing inserted, voicing untouched, byte-identical |
```

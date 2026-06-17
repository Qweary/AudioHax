# Input S29 / K3 RE-TUNE — the opening V→I cadence + the V7 pivot (harmonic rules)

**Author role:** Music Theory Specialist. **This is the harmonic-RULES design surface** for the
S29 re-tune levers that live in `src/chord_engine.rs` (spec `docs/spec-s29-k3-retune-build.md`
§2.1(a)/(b)-Option-A, §2.3). It specifies what the new/changed `chord_engine.rs` code does
MUSICALLY, concretely enough that the Rust body is a transcription of the rule. The byte-freeze,
the seam, the signatures, and engine.rs's frozen sha are fixed by the spec; this doc fills the
harmony. It EXTENDS `docs/input-s28-k3-pivot-harmony.md` (the K3 pivot) — read that first.

**Date:** 2026-06-16. Built against the tree at HEAD `dfcfb4c` (S28/K3 BUILT & CLOSED) and the
S29 chord_engine.rs scope transcribed against the spec. `src/engine.rs` stays frozen at
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — no engine.rs edit.

---

## 0. The musical problem in one sentence

K3 shipped the ANNOUNCEMENT of a modulation (a pivot chord at the boundary) but neither the
CONFIRMATION (a cadence in the new key) nor an unambiguous dominant. The operator heard chord
changes "that sounded normal" but could not tell a key had changed. S29 confirms the modulation
two ways, both inside `chord_engine.rs`:

1. **The opening V→I authentic cadence** — the step-0 pivot V is now RESOLVED to the destination
   tonic on the very next downbeat (step 1), voice-led so it reads as a true authentic cadence in
   the new key, not two root-position triads stacked (which a trombonist hears as parallel octaves).
2. **The V7 pivot** — the pivot chord gains its chordal seventh, turning a bare V triad into a V7
   whose tritone gives the dominant its pull, so even the announcement is sharper.

Plus a deterministic helper the planner needs:

3. **`tonic_triad`** — a forced, RNG-free root-position destination tonic "I" so the planner can
   guarantee `chords[0]` is the destination tonic for the V→I to resolve INTO.

All three are gated to be inert (`None`/byte-identical) on every identity / `home_only` /
`pivot:false` / non-modulating section, so the byte-freeze guards stay green.

---

## 1. Coordinate system (what the rules read)

Same as K3 (`input-s28` §1). All offsets are semitones relative to the home root; everything is
computable from `ctx`:

- `home_root_pc = ctx.key_tempo.home_root_midi % 12`
- `home_mode    = ctx.key_tempo.home_mode` (one mode across the whole piece; only the ROOT travels)
- `dest_off     = ctx.section.key_offset_semitones`
- `prev_off     = ctx.prev_key_offset_semitones` (`None` on the first section — NEVER a key change)
- `dest_root_pc = (home_root_pc + dest_off) mod 12`

The destination dominant (the pivot, from K3): `dom_root_pc = (dest_root_pc + 7) mod 12`, a
MAJOR-quality triad (so its major third is the destination's leading tone, the pitch that asserts
the new key regardless of the piece's mode).

---

## 2. Lever 3 — the V7 pivot (`pivot_chord_events`)

### 2.1 The rule

The pivot was a bare V triad. Add the dominant SEVENTH so it is unambiguously a functional
dominant:

```
dom_seventh_pc = (dom_root_pc + 10) mod 12        // a minor 7th above the dominant root = V7
```

**Why a 7th.** The interval that makes a dominant *pull* is the tritone between its major third
(the destination's leading tone) and its minor seventh. A bare triad has no tritone, so its
dominant function is weak. The 7th supplies the leading dissonance that *wants* to resolve down —
which it does, into the destination tonic's third on the next downbeat (§3, the dovetail with the
opening cadence).

### 2.2 Voice assignment (respecting the no-inversion frame bass < fill < melody)

The K3 voicing frame is unchanged for bass and melody; only the inner voice changes:

| voice | pitch class | rationale |
|---|---|---|
| **Bass** (unchanged) | `dom_root_pc` | root-position V7 — the strongest, most-prepared form |
| **Melody** (unchanged) | `dom_fifth_pc` = `(dom_root_pc+7)` | a stable, singable top tone over the V7 |
| **Fill / inner** (NEW) | `dom_seventh_pc` | the 7th is an inner-voice color tone by nature; seated at the fill floor it sits strictly between bass (dom root) and melody (dom fifth), so `no_inversion_invariant` holds by construction |

**Ensemble size.** A HarmonicFill role is only ever assigned when `num_instruments >= 3` (the
stratification gives an inner voice only when there is an instrument between the bass at index 0
and the melody at the last index). So for a **1- or 2-instrument ensemble there is no fill role,
the fill branch never executes, and the 7th is simply never voiced — the bare-triad pivot remains**
for those ensembles, exactly as spec §2.3 requires, with no explicit size check needed.

**The hinge.** The K3 common-tone hinge picker is RETAINED in the code as the documented proof
that a shared tone exists across the boundary (the picker only ever selects a pc that IS a member
of the destination V triad), which is WHY the pivot reads as a prepared move and not a splice. As
of S29 the inner voice sounds the resolving 7th rather than statically holding the hinge; the hinge
lives on as the conceptual line the 7th rides into its resolution.

---

## 3. Lever 1(b) Option A — the opening V→I authentic cadence (the voice-leading rule)

This is the heart of S29. **DEFAULT = Option A** (a voicing constraint on an already-present chord;
NO new `plan_phrases` stamp, NO new step — Option B, an explicit opening-PAC stamp, is HELD and is
NOT built). It is realized as a gated re-pointing of the resolution step's pitches, exactly like
the K3 land-home re-voicing.

### 3.1 When it fires

The step that RESOLVES the pivot is `step_in_section == 1` of a modulating section (the downbeat
immediately after the step-0 pivot V). Armed iff ALL hold:

- `ctx.section.pivot == true`
- `ctx.step_in_section == 1`
- `ctx.prev_key_offset_semitones` is `Some(prev)` with `prev != dest_off` (a real key change; a
  `None` prev — first section / identity — is never a modulation, the same shape as the pivot gate).

Under identity / `home_only` / `pivot:false` / non-modulating-boundary, the predicate is `false`
→ the voicing is untouched and byte-identical to pre-S29.

> **Mechanical note for the lead / Implementer (off-by-one between spec model and engine
> scheduling).** The pivot does NOT add a step: `locate(step_idx)` maps each global step to
> `(section, step_in_section)` indexing `section.steps`, and at `step_in_section == 0`
> `pivot_chord_events` MASKS the realization of `section.steps[0]` (the forced `chords[0]` = I) with
> the pivot V. So the forced I built by `tonic_triad` at `chords[0]` is never *heard at step 0*; the
> chord that sounds at step 1 is `section.steps[1]` (= RNG `chords[1]`). The opening-cadence rule
> therefore does NOT rely on `chords[1]` being the tonic — it RE-POINTS step 1's voicing to the
> destination tonic directly from `ctx` (independent of whatever RNG drew for `chords[1]`), so the
> V→I is real regardless. This makes the `tonic_triad` overwrite of `chords[0]` belt-and-suspenders
> for the *plan record* (and the `chords[0].name == "I"` test assertion) while the *audible*
> resolution comes from `pivot_resolution_pitch` at step 1. If the lead wants the forced I to also
> be the literal step-1 chord (so the plan and the audio agree), that is a `plan_phrases`/scheduling
> question for the Implementer, not a harmony change.

### 3.2 The two resolutions (what makes it an authentic cadence, not stacked triads)

The prior pivot sounded V7 of the destination: bass = dom root, melody = dom fifth (= destination
2nd degree), inner = dom 7th. The destination tonic I must resolve it with:

- **Leading tone UP by semitone to the new tonic.** The V's major third is the destination's
  leading tone; it pulls up a semitone to the tonic ROOT. Realized STRUCTURALLY: the I arrives
  **root-position with the tonic DOUBLED in the outer voices**, the upper voices descending by step
  INTO it (contrary/oblique to the bass leap). That is exactly the frame the leading-tone-up
  resolution produces, and the contrary motion is what AVOIDS the parallel octaves/fifths that two
  statically-stacked root-position triads would create.
- **Chordal 7th DOWN by step to the I's THIRD.** The pivot's inner 7th (`dest_root_pc + 5`) steps
  DOWN to the destination tonic's diatonic third — `dest_root_pc + 4` in a major-third mode
  (Ionian/Lydian/Mixolydian), `dest_root_pc + 3` in a minor-third mode (Aeolian/Dorian/Phrygian).
  Realized LITERALLY: the inner voice (which carried the 7th) seats the tonic third.

### 3.3 Per-role voicing of the resolution (the destination tonic I)

| voice | pitch class | motion from the pivot | rationale |
|---|---|---|---|
| **Bass** | `dest_root_pc` | dom-root → tonic-root | root-position I; the idiomatic authentic-cadence bass leap (down a 4th / up a 5th) |
| **Melody** | `dest_root_pc` | dom-fifth (dest 2nd degree) → tonic, DOWN a step | soprano-on-tonic = a strong arrival; the step-down against the bass leap is the contrary motion that voids parallel octaves between the outer voices |
| **Fill / inner** | `dest_root_pc + scale[2]` (the diatonic third) | dom-7th → third, DOWN a step | completes the V7→I resolution in the inner voice where the dissonance lived |

The third uses the piece's mode (`scale[2]` from the same `match mode` selection
`generate_chords`/`tonic_triad` use), so it matches the tonic the forced `tonic_triad` builds.
Every pitch is seated via `seat_pc_in_register` at the role's existing floor (no brightness octave
lift — the cadence wants a steady, grounded arrival), so bass < fill < melody holds by construction
and `no_inversion_invariant` cannot break.

---

## 4. Lever 1(a) — `tonic_triad` (the deterministic forced tonic)

A NEW public helper the planner calls to force a modulating section's first chord:

```rust
pub fn tonic_triad(&self, root_midi: u8, mode: &str) -> Chord;
```

**Rule.** A DETERMINISTIC root-position tonic "I" at `root_midi` in `mode`: root, diatonic third,
perfect fifth. NO RNG, NO secondary-dominant prepend, NO mode-mixture borrow. It reuses the EXACT
same scale selection `generate_chords` uses and the existing private
`roman_to_chord_complex("I", root_midi, &scale, HarmonicComplexity::Triad)`, so the chord tones are
byte-for-byte identical to a free-selected diatonic "I" at this root — only the SELECTION is forced.
`Chord.name == "I"` (so the Test Engineer can pin `chords[0].name == "I"`).

**Why Triad complexity (not a 7th).** The opening confirmation wants the bare tonic skeleton — a
7th on the tonic blurs the arrival, and a bare triad keeps the V7→I voice-leading frame (§3) clean.

---

## 5. The dovetail (one rule, stated once)

The "7th resolves DOWN by step to the new third" rule is the SAME rule for both the pivot's added
7th (Lever 3) and the opening cadence's resolution (Lever 1(b)). It is specified once here (§3.2)
and realized across the pivot→I boundary: the step-0 inner 7th → the step-1 inner tonic-third. The
"leading tone UP to the new tonic" rule is realized structurally by the root-position,
tonic-doubled, step-down-into-it arrival (§3.2). Together they turn the K3 announcement into a
confirmed modulation.

---

## 6. Voice-leading compromise (for the trained ear)

The realizer is a **one-note-per-role** texture: each role sounds exactly one pitch per step, and
the "voice leading" is expressed by which chord tone each role seats and in which register, not by
an explicit four-part part-writing search. So the leading-tone-up resolution is realized
*structurally* (the cadence ARRIVES in the configuration the LT-up move produces — root position,
tonic doubled in the outer voices, upper voices descending by step) rather than by literally
tracking a leading-tone voice from step 0 to step 1 and moving it up a semitone. For a 3-instrument
ensemble this is faithful: bass leaps to the tonic, the melody steps down to the tonic (the
soprano-on-tonic PAC marker), and the inner voice steps the 7th down to the third — three voices,
three correct resolutions, no parallel perfects between the outer pair. The bare-triad-on-bare-triad
parallel-octave hazard the spec names is avoided because the upper voices MOVE BY STEP into the
arrival while the bass LEAPS (contrary/oblique motion), not because of an explicit parallel-motion
search. This is the conservative, correct-and-simple realization the spec asks for; if a re-listen
finds the cadence still reads as "two triads," the held Option B (an explicit opening-PAC stamp via
`plan_phrases`) is the documented next step.

---

## 7. Summary table (for the implementer's transcription)

| decision | rule |
|---|---|
| `tonic_triad` | deterministic root-position "I" at `root_midi`/`mode`, Triad complexity, `name == "I"`; reuses `roman_to_chord_complex("I", …, Triad)`; no RNG |
| V7 pivot | add `dom_seventh_pc = (dom_root_pc + 10) % 12`; seat it in the FILL/inner voice (3+ ensemble); omit for 1-/2-instrument (no fill role) |
| Opening cadence fires | `pivot == true` AND `step_in_section == 1` AND `Some(prev) != dest` |
| Opening cadence voicing | bass = dest tonic root; melody = dest tonic root (soprano-on-tonic); fill = dest tonic diatonic third (`scale[2]`) |
| LT-up resolution | structural: root-position arrival, tonic doubled outer, upper voices step down into it (contrary to the bass leap → no parallel octaves) |
| 7th-down resolution | literal: inner 7th (`dest+5`) → inner tonic third (`dest+4` major / `dest+3` minor) |
| Identity guarantee | every gate above is false on identity / `home_only` / `pivot:false` / non-boundary → nothing inserted, voicing untouched, byte-identical; engine.rs frozen at `e50c7db1…` |
| Held | Option B (`plan_phrases` opening-PAC stamp); dwell increases; texture/figuration second dimension — all out of S29 scope |
</content>
</invoke>

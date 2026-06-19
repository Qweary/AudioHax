# S42 Salience Diagnosis — Synthesis & Build Work Order

DESIGN / DIAGNOSIS DOCUMENT. No production code was changed to produce it. This
is the synthesis of four diagnostic inputs that all read from the same empirical
evidence base:

- `docs/design-s42-trace.md` — the per-role realized-NoteEvent ground-truth trace
  (the evidence), the signal-flow code trace, and per-lever freeze verdicts.
- `docs/design-s42-theory.md` — the music-theory / composition-craft lens.
- `docs/design-s42-aesthetics.md` — the whole-piece songwriting-aesthetics lens.
- `docs/design-s42-affect.md` — the perceptual / cross-modal affect lens.

It converts the unanimous finding into one ranked recommendation and a concrete,
freeze-verified build work order. The realization kernel `src/engine.rs` is
**byte-frozen** at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
and is **not touched by anything proposed here** — every edit lands in
`assets/mappings.json` (selection tables) and, optionally, in one line of
`src/chord_engine.rs` (a velocity role-bias arm), both of which the frozen kernel
only *calls*.

---

## 1. Diagnosis

The trained-ear verdict was unambiguous: `example.jpg` (carries a theme) and
`Lena.png` (carries no theme) were heard as **"the same piece in a different
key,"** and theme-bearing could not be told from theme-less. The trace shows
exactly why, and all three analytical lenses converge on the same mechanism.

**The melody is buried by the mix and by figuration, not by register.** The
melody is the highest voice on both renders (median MIDI 69 / 82, well above
HarmonicFill ~61/64, Pad ~62/63, Bass ~32/42; trace A.1/A.2). Register is the one
salience cue that *passes*. But it is the weakest stream-segregation cue, and
every other cue says "same stream":

- **Loudness is inverted.** The melody's velocity median (88 / 81) is level with
  the Pad (82 / 88) and *below* the HarmonicFill (101 / 94). The **HarmonicFill is
  the loudest sustained role on both renders.** The melody carries only its fixed
  `+2` role bias (`src/chord_engine.rs:1372`) while the Fill falls through the
  role match with no negative bias (`:1380`, `_ => {}`), so the inner voice floats
  to the top of the dynamic field. The foreground voice is quieter than a
  background voice.
- **Rhythm is doubled, not independent.** The melody shares the bed's onset grid.
  Its richest figure — the even 4-onset arpeggio `(0,156,312,468)`
  (`:1912-1924`) — is rhythmically identical to the Pad's alberti/broken-chord
  burst `(0,156,313,469)` (`:1797-1799` → `figured_bed`, `:2278`). Two voices on
  one onset grid fuse into a single stream. The melody is doubling the bed's
  rhythm an octave up, not counterpointing it.

**The accompaniment is the perceived identity.** The loudest, most-repeated layer
is the bed — sustained Bass + sustained Fill + a near-ostinato Pad broken-chord
burst (32 of 42 steps on `example`). A listener identifies a piece by its most
salient, most-repeated surface layer; here that layer is the bed, and the bed
reads only the single global `edge_activity` scalar (`:1504-1515`, with the
`(section.density − 0.5)` term pinned to 0 because `section.density == 0.5` on
every step) and a coarse per-section figuration profile — **neither encodes the
theme.** So the layer the ear is using to identify the piece is exactly the layer
that carries no per-image identity.

**The dormant prominence system is the root cause.** A complete
melody-foregrounding apparatus EXISTS, is freeze-safe, and is fully built: the
`subject_melody` profile in `assets/mappings.json` would set Melody weight 1.0
(→ `(prominence_w − 0.5) * PROMINENCE_VEL_SPAN` becomes +9 velocity instead of 0
at `:1390-1392`), add +2 register, lower the melody's `edge_activity` rhythm-band
cutoffs via `prom_shift` (`:1910-1911`, so the melody subdivides on a *different*
grid than the bed and breaks the lockstep), and recess Pad (0.3) and Fill (0.4)
below neutral. It is **gated off**: its rule fires only when
`subject_size ∈ [0.05, 0.55]` AND `fg_bg_contrast ≥ 0.25`. Both images pass the
size gate but **fail the contrast gate** (`example` 0.136, `Lena` 0.052 vs the
0.25 floor; trace B.2). With no rule matching, prominence resolves to the empty
`uniform` default → neutral 0.500 on every step → every saliency nudge is a no-op.
That single threshold is why the melody is dynamically indistinct.

**`example` and `Lena` differ only in key audibly.** The real differences are
(a) key/register offset, (b) one Pad figuration choice (animated broken-chord bed
vs plain block triad), and (c) theme presence in the melody pitch content. Of
these, (a) is "a different key," (c) is inaudible *because of the salience
failure* (a real motif rides at bed level, so swapping it for a free-selected top
note changes nothing isolable), and (b) is a quiet inner-voice detail. The only
difference that survives to the surface is the one that says the least.

**Relation to the S13 composition-architecture concern.** This is the S13 "no
macro-form / image-as-whole understanding" worry resurfacing — the system still
has only note-level craft, and the operator is right that the pieces lack a heard
identity. But the S42 evidence points somewhere narrower than the S13 framing
implied: **the foregrounding machinery already exists and is merely gated off by
one miscalibrated threshold.** The fix is a small incremental lever (a
selection-table retune), **NOT a rewrite.** The scan-sonifier-to-composer
re-architecture remains a separate, later question; it is not what this defect
requires.

---

## 2. Ranked Recommendation

All three analytical lenses (Theory, Aesthetics, Affect) INDEPENDENTLY rank these
identically.

### RANK 1 — SALIENCE (foreground the melody over the bed). Decisive.

This is the only family that moves *what the ear treats as the subject*. It
attacks the root cause directly and does **triple duty** with a single change:

1. **Guaranteed foreground.** It gives the ear a figure on every image, fixing the
   "I can't find a line to evaluate" failure.
2. **Per-image divergence.** It makes the theme/no-theme difference — the single
   largest per-image structural difference the trace found — finally *audible*:
   once the melody is the subject, `example`'s recalled motif reads as a recurring
   hook and `Lena`'s motif-less free-selection reads as a wandering exposed line.
   Two different kinds of piece, not "same piece, different key."
3. **Freeze-safe.** It rides the existing `prominence` / `prominence_catalogue`
   shape the loader already parses; the frozen kernel only consumes the resolved
   weights.

**The concrete form: a two-tier prominence scheme** (the Affect lens's refinement,
endorsed in substance by Theory and Aesthetics):

- **(a) An always-on `melody_forward` DEFAULT profile** — a mild, guaranteed
  melody lift applied to *every* image. This is the load-bearing half: it
  guarantees a foreground regardless of whether any gate fires. There is no
  aesthetic case for a neutral-everything mix as the fallback — tonal music has a
  figure; "uniform / no figure" should be a rare earned state, not the default
  both real photos fall into.
- **(b) Demote the miscalibrated `fg_bg_contrast` gate** from "turn foreground ON
  at all" to merely "ESCALATE to the full `subject_melody` lift," relaxing its
  floor from `0.25` to `0.10`. Now `example` (0.136) *earns* the full lift while
  `Lena` (0.052) gets the milder default. This yields guaranteed foreground PLUS
  a second per-image distinction (how *assertively* the melody leads), layered on
  top of the theme/no-theme difference.

**Pair it with a one-line negative HarmonicFill velocity bias** (Theory lens):
the Fill is currently the loudest role, the single biggest competitor masking the
melody. The `melody_forward` default already recesses Fill via its weight, so this
is belt-and-suspenders correctness, not strictly required — but it is the cheapest
companion fix and worth doing in the same pass.

### RANK 2 — ACCOMPANIMENT VARIATION (stop pieces sharing a gait). Subordinate, with the operator's trap warning.

All three lenses flag this as the operator's **explicitly warned-against trap**,
and the trace supplies the proof: **the beds ALREADY differ** — `example` got the
animated broken-chord/alberti bed, `Lena` fell to the plain block triad (trace
A.1/A.2, B.1). The operator heard them as the same piece anyway. That is direct
evidence that making a low-attention layer differ *more* will test green (the
figures demonstrably diverge) and sound identical (the ear is not listening to the
comping). The discriminator between a real fix and a fake one is: *does it change
what the ear treats as the subject?* Bed-shuffling does not.

Accompaniment variation is a genuine *second-order* refinement — a varied bed
under an audible hook deepens per-image character — but it only pays off once a
foreground exists for it to sit behind. **Defer to a later slice.**

### Why "widen theme presence" is now subordinate.

Giving more images a real theme is the natural high-value follow-up, but it is
subordinate **for the same reason and in the same order**: widening theme presence
*before* the melody is foregrounded adds content to an inaudible layer — it tests
green (more images carry a theme) and sounds identical (the theme still rides at
bed level). The correct sequencing is **foreground first, then theme.** Turn the
foreground ON (identity), then make the foreground worth hearing (give more images
a real theme). Until salience lands, widening theme presence is exactly the
"inaudible layer" trap in a different costume.

---

## 3. Concrete S43 BUILD Work Order

All edits below are freeze-verified against the frozen kernel. Current confirmed
state of the two tables in `assets/mappings.json`:

```jsonc
// composition.prominence_catalogue  (current)
[ { "id": "uniform", "layers": [] },
  { "id": "subject_melody", "layers": [
      { "role": "Melody", "weight": 1.0 }, { "role": "CounterMelody", "weight": 0.6 },
      { "role": "HarmonicFill", "weight": 0.4 }, { "role": "Pad", "weight": 0.3 },
      { "role": "Bass", "weight": 0.5 } ] } ]

// composition.prominence  (current)
{ "default": "uniform",
  "rules": [ { "when": [ {"knob":"subject_size","op":"in_range","lo":0.05,"hi":0.55},
                         {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ],
              "pick": "subject_melody" } ] }
```

### Edit 1 (REQUIRED) — add the always-on `melody_forward` default profile

Under `composition.prominence_catalogue`, ADD this row (keep `uniform` and
`subject_melody` unchanged):

```json
{ "id": "melody_forward", "layers": [
    { "role": "Melody",        "weight": 0.78 },
    { "role": "CounterMelody", "weight": 0.58 },
    { "role": "HarmonicFill",  "weight": 0.40 },
    { "role": "Pad",           "weight": 0.40 },
    { "role": "Bass",          "weight": 0.50 } ] }
```

Starting values, with rationale:
- **Melody 0.78** — clear-but-not-shouting. At weight 0.78 the centered nudge
  `(0.78 − 0.5) * PROMINENCE_VEL_SPAN(18)` ≈ **+5 velocity**, stacked on the
  existing `+2` role bias ≈ **+7 over a neutral bed** — a real, audible
  figure/ground gap — plus the register lift and the lowered rhythm-band cutoffs.
  Deliberately below the full `subject_melody` 1.0 so the escalation tier still
  reads as MORE foregrounded when it fires (this two-tier gap is itself a source of
  per-image identity).
- **HarmonicFill 0.40 / Pad 0.40** — recessed below neutral so the bed (and the
  currently-over-loud Fill) sits under the line. Both stay `> 0.25` so the
  accompaniment supports rather than vanishes.
- **Bass 0.50** — neutral; the structural foundation neither lifts nor recedes.
- **CounterMelody 0.58** — mild lift (no CounterMelody on either S42 render, so
  inert here, but consistent with the foreground intent for layer sets that have
  one).

### Edit 2 (REQUIRED) — make foreground the default and demote the gate to escalation

Under `composition.prominence`, CHANGE the default and relax the gate floor:

```json
{ "default": "melody_forward",
  "rules": [ { "when": [ {"knob":"subject_size","op":"in_range","lo":0.05,"hi":0.55},
                         {"knob":"fg_bg_contrast","op":"ge","lo":0.10,"hi":0.0} ],
              "pick": "subject_melody" } ] }
```

Two changes: `"default": "uniform"` → `"default": "melody_forward"` (the
load-bearing change — guarantees a foreground on every image), and the gate floor
`"lo":0.25` → `"lo":0.10` (the escalation recalibration — `example` 0.136 now
earns full `subject_melody`, `Lena` 0.052 stays on the `melody_forward` default).

### Edit 3 (OPTIONAL, one line) — give HarmonicFill a small negative velocity bias

The Fill currently falls through the `realize_velocity` role match with no bias
(`src/chord_engine.rs:1380`, the `_ => {}` arm), which is why it ends up the
loudest role. Add a HarmonicFill arm with a modest negative bias (between the
Bass's `−1` and the Pad's `−3`; the inner held tone should sit under the melody
but need not recede as far as the Pad). The edit is a pure addition to the
existing `match role` block at `src/chord_engine.rs:1371-1381`:

```rust
// before: HarmonicFill falls through `_ => {}` and keeps full level.
// add an arm so the inner held voice recesses under the foreground line:
OrchestralRole::HarmonicFill if !is_cadence => vel -= 2.0,   // starting value, ear-tunable
```

Belt-and-suspenders: `melody_forward` already recesses Fill to 0.40, so Edit 3 is
optional. Recommendation: **ship Edits 1+2 first, re-listen, and only add Edit 3
if the Fill is still competing.**

### Per-edit FREEZE verdict — NONE touch frozen `engine.rs`

| Edit | Site | Touches `engine.rs`? | Verdict |
|---|---|---|---|
| 1 — `melody_forward` catalogue row | `assets/mappings.json` → `prominence_catalogue` | NO | **FREEZE-SAFE — JSON only.** Additive Vec row; loader already parses the shape. |
| 2 — default + gate floor | `assets/mappings.json` → `prominence` | NO | **FREEZE-SAFE — JSON only.** String default + one numeric floor; no schema change. |
| 3 — Fill velocity bias | `src/chord_engine.rs:1371-1381` | NO | **FREEZE-SAFE.** Pure addition to the `realize_velocity` role match in `chord_engine.rs`; guarded `!is_cadence` so cadence goldens stay byte-stable. |

The frozen kernel only *calls* `realize_step`/`realize_velocity` and resolves
sections; the planner (`src/composition.rs:1543-1545`) already resolves
`prominence` from this table and the realizer already consumes the resolved
weights. Expected runtime effect: both S42 renders move from resolved Melody
prominence 0.500 (neutral) to a real lift — `example` to `subject_melody` (1.0),
`Lena` to `melody_forward` (0.78). `src/engine.rs` sha256 stays
`e50c7db1…2348261`. The byte-frozen legacy/identity render path is untouched
(the `uniform` and `subject_melody` profiles are unchanged; only which profile a
non-subject image *resolves to* changes).

### What the S43 taste gate should listen for

Run the Affect and Aesthetics review voices in parallel as a standing taste gate
beside correctness, on the A/B `--seed 42` renders of `example.jpg` and
`Lena.png`. The fix worked if the ear confirms:

1. **The melody now reads as the subject** — it is the loudest, clearest voice; the
   HarmonicFill no longer floats to the top; there is a tune to follow, not a top
   note of the chords.
2. **`example` and `Lena` now sound like different pieces, not just different
   keys** — `example`'s recalled theme reads as a recurring hook; `Lena`'s
   foreground reads as a wandering line with no home. The two diverge in *how
   assertively* the melody leads (full lift vs. default) on top of the theme/
   no-theme difference.
3. **The bed recedes but does not vanish** — the accompaniment still supports;
   removing the Fill's over-loudness has not hollowed out the texture.
4. **The freer foregrounded melody still lands on chord tones and resolves at
   cadences** (Theory watch-item — a louder voice exposes voice-leading roughness
   that was forgivable when buried), and the cadential homecoming is heard *in the
   melody*, not only in the harmony bed.

Encodable correctness guards to pair with the ear test (from the Aesthetics lens):
resolved Melody prominence `> 0.5` on every section of every render
(foreground-exists invariant); the melody's biased velocity ≥ the loudest bed
role's on accented steps (audible figure/ground gap); `subject_melody.Melody (1.0)
> melody_forward.Melody (0.78)` (two-tier preserved); Pad/Fill weights below
neutral but `> 0.25` (bed recedes, does not vanish); and the existing byte-freeze
test on `src/engine.rs` still passes.

---

## 4. Open Calibration Risk

- **The always-on default Melody weight (~0.78) is placeholder-grade.** The
  `0.78 / 0.58 / 0.40 / 0.40 / 0.50` split is a judgment call sized between neutral
  (0.5) and the full `subject_melody` profile (1.0). The *direction* (melody
  loudest, Fill/Pad recessed below neutral) is high-confidence — loudness/level is
  a primary stream-segregation cue — but the exact magnitude wants the operator's
  ear. If the figure is too timid, raise it toward 0.85; if it shouts, lower toward
  0.70. Same for the optional Fill bias (`−2` starting value).
- **The escalation threshold (0.10) is fitted to a two-image sample.** `0.10`
  cleanly splits the two S42 photos (`example` earns full, `Lena` gets default),
  but it should be re-checked against a wider image set so the full
  `subject_melody` escalation fires for genuinely subject-dominant images and not
  for cluttered ones. If only one threshold can be trusted, the
  **default-to-foreground change is the load-bearing half and is robust on its
  own**; the escalation gate is the refinement.
- **The real-photo-clustering pathology is why the original 0.25 gate was
  miscalibrated.** `fg_bg_contrast` (a clamped sum of normalized value/saturation/
  edge differences between the argmax "subject" cell and the border ring) was
  thresholded as if it ranged uniformly over [0,1], but real photographs are
  spatially autocorrelated — adjacent regions resemble each other, so cell-vs-ring
  differences cluster in the bottom quartile. The measured 0.136 / 0.052 are not
  outliers; they are where the real-photo distribution lives. **This is the
  identical pathology already flagged for the rhythm-cell band edges (DP-A): a
  threshold calibrated against the theoretical range, not the realized real-image
  distribution, so the interesting branch never fires.** The two-tier scheme
  side-steps the pathology by making foreground the default rather than gating it
  on a fragile statistic; the relaxed escalation gate is a recalibration to the
  realized distribution, not a fix to the underlying clustering. A genuinely robust
  figure-ground saliency (knowing *what* the subject is) is an opt-in ML
  segmentation tier for later — the lever here does not need it.
- **Scope discipline:** this lever is scoped to the `prominence` table only. Do
  NOT relax the *other* `fg_bg_contrast ≥ 0.25` gates (key_scheme, texture, form)
  in the same pass — they guard different musical decisions and have their own
  realized-distribution questions. Widening them is a separate, evidence-gated
  change.

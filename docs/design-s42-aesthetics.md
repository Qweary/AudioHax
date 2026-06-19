# S42 Aesthetics Diagnosis — Where Is the Piece's Identity, and the Smallest Lever to Get It Back

DESIGN / DIAGNOSIS DOCUMENT. No code changed. This is the composition /
songwriting-aesthetics reading of the S42 ground-truth trace
(`docs/design-s42-trace.md`) and the operator's "two pieces, same song in a
different key" verdict. It answers three questions through the whole-piece
"does it cohere and have a distinct identity" lens — NOT through chord internals
(harmony lens) and NOT through pixels (extraction lens). It ends in a ranked
recommendation and the single smallest lever that makes `example` and `Lena`
read as different *pieces*.

The trained-ear gate is the verdict to satisfy: the listener could not tell a
theme-bearing piece from a theme-less one, and heard, across everything, a
pleasant chord bed under one dominant dotted/arpeggio gait. That verdict is
correct, and the trace explains exactly why.

---

## 1. Where is the piece's IDENTITY actually coming from today — and where SHOULD it come from?

**Today, identity comes almost entirely from the accompaniment bed, and the bed
is nearly the same bed every time.** The trace is unambiguous: on both images the
four sounding roles are Bass, HarmonicFill, Melody, Pad; the loudest sustained
voice on BOTH renders is the inner HarmonicFill (velocity median 101 / 94), and
the melody sits at or below the bed (88 / 81) despite being in the highest
register. The perceptual subject — the thing the ear locks onto as "the piece" —
is therefore the sustained Bass + sustained Fill + a Pad broken-chord/block bed.
That bed is what the listener is calling "the song," and it is built from a small,
shared vocabulary that barely moves between two unlike images. So the de-facto
song is the accompaniment, and it is largely the same accompaniment in two keys.
That is the whole problem, stated in one sentence.

**This is the classic shapeless-piece failure: a piece with no foreground has no
identity, only a key.** From a songwriting standpoint, a piece's identity is its
*hook* — the one figure a listener could hum back, the thing that is unmistakably
THIS piece and not another. There is no hook in the heard surface today. The
melody voice contains the only material that actually differs per image at the
level that matters (`example` carries a real stated/recalled motif; `Lena` has no
theme and free-selects the top chord tone on every step) — but that voice is
dynamically level with, and quieter than, the bed, so the ear never separates it
as a figure. The one channel that carries per-image identity is mixed into the
texture and lost.

**Where identity SHOULD come from:** the foreground. In a short generated piece,
the hook is the melody — its contour, its register, its rhythmic freedom relative
to a steadier bed, and (where present) its returning theme. The bed's job is to
*support and contrast* the hook, not to BE the piece. Right now those roles are
inverted: the bed is the subject and the would-be hook is wallpaper. Identity has
to move to the foreground voice, because that is the only voice whose content the
system already varies per image in a way a listener could, in principle, latch
onto and remember.

---

## 2. Foreground vs. variation — which gives a piece a distinct identity?

**A distinct, foregrounded HOOK is what makes songs recognizable, and it is the
thing that is missing.** Ask what lets anyone tell two songs apart: it is almost
never the comping pattern under the tune — it is the tune. Listeners identify a
song in two seconds by its melodic hook, not by whether the left hand is playing
alberti or block chords. The whole craft of songwriting treats the foreground
line as the carrier of identity and the accompaniment as the frame. So between the
two fix families, **foregrounding the melody is the one that creates per-image
identity**, because it promotes the already-per-image-varying voice (theme vs.
free-select, plus the dotted/syncopated/arpeggio melodic figures the trace shows
the melody already generating) from inaudible to audible. The moment the melody is
the loudest, highest, rhythmically-freest voice, `example`'s real recalled motif
becomes a *heard recurring hook* and `Lena`'s motif-less free-selection becomes a
*heard wandering line with no home* — and those two now sound like different
pieces, because their actual difference (one has a theme, one doesn't) is finally
on the surface where the ear weights it most.

**Accompaniment variation alone does NOT confer identity — and worse, it risks
re-committing the exact regression the operator warned about.** The trace shows
the texture system is already somewhat discriminating: `example` picked an animated
broken-chord/alberti bed while `Lena` fell to the plain block-triad default, so the
two beds *already* differ. The operator heard them as the same piece ANYWAY. That
is the proof: making the bed differ MORE does not make the pieces differ
perceptibly, because the bed is a low-attention layer. Pouring effort into bed
variation is "adding more of an inaudible layer" — it will test green (the figures
demonstrably differ) and sound identical (the ear isn't listening to the comping).
This is precisely the trap the operator flagged about "widen theme presence": any
fix whose payoff lives below the attention threshold is a non-fix. The discriminator
between a real fix and a fake one is: *does it change what the ear treats as the
subject?* Foregrounding does. Bed-shuffling does not.

**Critical caveat — foregrounding only confers identity if the foregrounded
content actually differs.** Promoting the melody is necessary but works only because
the melody content already varies per image. For `example` (has a theme) the win is
real and immediate: a recalled motif becomes a recognizable hook. For `Lena` (no
theme) foregrounding makes audible a wandering top-tone line — which still reads as
*a different kind of piece* (aimless vs. anchored), so identity-discrimination is
achieved even there. But this surfaces a real downstream tension: a theme-less piece
with a loud aimless foreground may sound *exposed* rather than *pleasing*. Foreground
is the identity lever; whether every image deserves a stated theme is the next
question after it (see §5 and §6). The right order is: turn the foreground ON first
(identity), then make the foreground worth hearing (give more images a real theme).

---

## 3. Ranked fix families + the single smallest identity-making lever

### Ranking, for IDENTITY / MEMORABILITY

**RANK 1 — SALIENCE (foreground the melody over the bed).** This is the family that
attacks the root cause the trace isolates: the melody is in the right register but
dynamically buried, the whole foregrounding apparatus exists and is freeze-safe, and
it is simply *gated off* for these two images by one threshold. Activating it
promotes the one voice whose content already carries per-image identity. It is the
only family that moves what the ear treats as the subject. Highest identity leverage,
lowest risk, smallest change.

**RANK 2 — ACCOMPANIMENT VARIATION (stop pieces sharing a gait).** Useful as a
*secondary* texture refinement once a foreground exists — a varied bed under a clear
hook deepens character. But as a primary identity fix it is the warned-against trap:
it changes a low-attention layer and will not move the operator's verdict. It earns
its keep only after Rank 1 has given the ear a subject to hang the bed-variation
contrast against. Deprioritize for this slice.

### The single smallest identity-making lever

**Relax the `prominence` gate so the dormant `subject_melody` profile actually
fires on these images — a JSON-only change in `assets/mappings.json`, no Rust, no
touch to the frozen kernel.** Concretely:

The current rule (`composition.prominence.rules`) requires BOTH
`subject_size ∈ [0.05, 0.55]` (both images PASS) AND `fg_bg_contrast ≥ 0.25` (both
images FAIL: `example` 0.136, `Lena` 0.052). Because no rule matches, prominence
falls to the `uniform` default, whose `layers` is empty, which the planner resolves
to neutral 0.5 on every step, which makes every saliency nudge a no-op. That single
`fg_bg_contrast ≥ 0.25` floor is the entire reason the melody is dynamically
indistinct.

The smallest lever is to **make a melody foreground the floor behavior, not an
exceptional one** — i.e. change the *default* prominence from `uniform` to a melody
foreground, so the question becomes "how foregrounded," never "is there a foreground
at all." Aesthetic rationale: a generated short piece with NO foreground is the
shapeless-piece anti-pattern; there is no aesthetic case for a neutral-everything
mix as the fallback. A song should always have a singer.

Two equally-small JSON expressions of this, in order of preference:

**Option A (preferred) — add a gentle always-on default foreground profile and make
it the prominence default.** This guarantees every image gets a foreground while
reserving the strong lift for the genuinely subject-dominated images that pass the
existing gate. Add one catalogue row and change one default string:

```jsonc
// composition.prominence_catalogue — ADD this row:
{ "id": "melody_forward", "layers": [
    { "role": "Melody",        "weight": 0.78 },
    { "role": "CounterMelody", "weight": 0.58 },
    { "role": "HarmonicFill",  "weight": 0.42 },
    { "role": "Pad",           "weight": 0.40 },
    { "role": "Bass",          "weight": 0.50 } ] }

// composition.prominence — CHANGE the default, KEEP the existing strong rule:
"prominence": {
  "default": "melody_forward",          // was "uniform"
  "rules": [
    { "when": [ {"knob":"subject_size",  "op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ],
      "pick": "subject_melody" }         // unchanged: strong lift for true subject images
  ]
}
```

Why `0.78` and not `1.0` for the default Melody weight: `subject_melody` (weight
1.0) is the *maximal* lift reserved for an image with a clear, sized, high-contrast
subject — a portrait whose face dominates the frame earns a fully spotlit melody.
The everyday default should be a clear-but-not-shouting foreground: enough to make
the melody unambiguously the subject (the saliency velocity span is +18 at weight
1.0, so 0.78 ≈ +5 velocity over neutral, stacked on the existing +2 role bias ≈
+7 total over the bed — a real, audible figure/ground gap — plus the register and
rhythm-cutoff nudges), while leaving headroom so the strong `subject_melody` rule
still reads as MORE foregrounded when it fires. This preserves a *two-tier* dynamic
between "ordinary piece" and "portrait/subject piece," which is itself a source of
per-image identity.

**Option B (smaller still, but coarser) — relax the existing rule's contrast floor
from `0.25` to `~0.10`.** One number change, no new row:

```jsonc
{"knob":"fg_bg_contrast","op":"ge","lo":0.10,"hi":0.0}   // was 0.25
```

This fires `subject_melody` (the full 1.0 lift) for `example` (0.136 passes) but
still NOT for `Lena` (0.052 fails). Aesthetically that is acceptable — and even
revealing: it makes the theme-bearing `example` jump to a fully foregrounded hook
while the theme-less low-contrast `Lena` stays neutral, so the two diverge HARD and
the operator's exact A/B finally separates. The downside is that it leaves
genuinely-foregroundless pieces (anything below 0.10 contrast, like `Lena`) still
shapeless, so it fixes the A/B comparison without fixing the general "every quiet
image is mush" failure. **Option A is the better aesthetic default; Option B is the
minimal proof-of-concept if the goal is purely to make THESE two diverge with the
absolute fewest edits.**

I recommend **Option A** as the v1 slice: it is still JSON-only, still freeze-safe,
guarantees every piece has a hook, preserves the strong-lift tier for true subject
images, and directly answers the operator's verdict by moving the heard subject from
the bed to the tune.

---

## 4. Guard-rails (encodable, testable "pleasing" properties for this slice)

These keep the foreground lever from producing foreground-valid-but-ugly output and
make the fix falsifiable by listening:

1. **Foreground-exists invariant.** For every rendered section, the resolved Melody
   prominence weight is `> 0.5` (strictly above neutral). No piece ships with a
   neutral-everything mix. *Testable:* assert resolved `prominence` for Melody `> 0.5`
   on every section of every render.
2. **Audible figure/ground gap.** The Melody's effective velocity ceiling exceeds the
   loudest accompaniment role's by a minimum margin (target ≥ +4 after all biases), so
   the melody is never quieter than the HarmonicFill again. *Testable:* per-step, the
   melody's biased velocity ≥ max(bed roles' biased velocity) on accented steps.
3. **Foreground varies per image.** Two distinct images produce melody lines that
   differ in ≥1 of {has-theme, contour, rhythmic-band distribution} AT THE
   FOREGROUNDED dynamic level — i.e. the difference lands in the loud voice, not a
   buried one. *Testable:* the A/B `example` vs `Lena` melody event streams differ in
   theme-presence and in their realized rhythm-band histogram.
4. **Two-tier foreground preserved (Option A).** An image passing the
   `subject_melody` gate yields a strictly higher Melody weight than the
   `melody_forward` default. *Testable:* `subject_melody.Melody (1.0) >
   melody_forward.Melody (0.78)`.
5. **Bed recedes, does not vanish.** Pad/Fill weights drop below neutral but stay
   `> 0.25`, so the accompaniment still supports rather than disappearing (a
   foreground with no frame is as shapeless as a frame with no foreground).
6. **Legacy/identity path byte-frozen.** The `identity` texture profile and the
   no-theme/`uniform`-equivalent legacy render path are unchanged; `src/engine.rs`
   sha256 stays `e50c7db1…2348261`. *Testable:* existing byte-freeze test on the
   legacy path still passes; this change touches only the `pad_bed`-family prominence
   resolution.

---

## 5. Sliceability — the v1-essential audible win vs. later refinement

**v1-essential (this slice, one audible win):** activate the foreground. Ship
Option A (melody-forward default + preserved strong-lift rule). The single audible
result the operator should hear: the melody steps forward as the subject, the bed
recedes to support, and `example` (recalled theme → recurring hook) now sounds
*different from* `Lena` (no theme → wandering exposed line), not "the same piece in
another key." That is the whole win, and it is one JSON edit.

**Later refinements, explicitly OUT of this slice:**
- **Give more images a real theme (Affect + Theory lenses).** Once the foreground is
  loud, a theme-less foreground sounds exposed. The high-value follow-up is widening
  *theme presence* — but ONLY behind the now-audible foreground (the operator's warning
  stands: widening theme presence WITHOUT first foregrounding tests green and sounds
  identical, because the theme stays buried). Foreground first, then theme. This is the
  correct sequencing the operator's redirect was pointing at.
- **Accompaniment variation (Rank 2 family).** Retune the `texture` rules / add
  figuration rows so the bed under the hook contributes secondary character. Worthwhile
  AFTER a foreground exists; pointless before. Defer.
- **Per-image bed busyness.** Drive `section.density` off the image (it is pinned at
  0.5 today) so the bed's activity tracks the image — a `composition.rs` planner change,
  freeze-safe, but a Rank-2 deepener, not an identity fix. Defer.
- **Form/key-plan identity (the macro shape).** Both images currently realize a
  rounded/ternary excursion but the trace shows section density and key plan are not yet
  doing per-image dramatization work. Binding the key plan to section ROLES and to affect
  (bright image lifts to V and comes home; dark image sinks to the relative) is the next
  whole-piece identity layer after foreground — but it is a separate slice with its own
  Theory-lens dependencies (pivots, cadential homecoming). Defer.

---

## 6. Risks / open tensions for the Music Theory and Affect lenses

1. **(Affect) Exposed theme-less foreground.** Foregrounding a no-theme image (like
   `Lena`) makes a wandering top-chord-tone line LOUD. Identity is achieved, but
   pleasingness may suffer — a loud aimless line can sound worse than a buried one. The
   Affect lens should weigh whether the no-theme free-selection needs a gentler foreground
   weight than the theme-bearing case, or whether the real answer is the §5 follow-up
   (give it a theme). Flag, not block: identity is the priority for this slice; polish is
   the next one.
2. **(Theory) The freer melody rhythm at higher prominence.** The `subject_melody`/
   `melody_forward` lift lowers the melody's `edge_activity` rhythm-band cutoffs (the
   `prom_shift`), so a foregrounded melody subdivides more freely. Theory should confirm
   the freer arpeggio/syncopation bands still land on chord tones and resolve at cadences
   — a louder voice exposes any voice-leading or non-chord-tone roughness that was
   forgivable when buried.
3. **(Theory) Cadential homecoming under a loud melody.** With the melody now the
   subject, the RETURN/cadence must be heard *in the melody*, not just in the bed —
   otherwise the foregrounded line can wander past the structural homecoming and rob the
   ending of resolution. Theory should confirm the cadence realization reaches the melody
   voice, not only the harmony bed.
4. **(Shared-file) `mappings.json` is single-writer with the Music Theory lens.** The
   `prominence_catalogue` row + the `prominence.default` change below must be merged by
   the single mappings.json writer, coordinated with Theory (who owns the harmonic/
   progression/extension tables in the same file). I author the rows; one writer commits.

---

## Appendix — exact mappings.json rows to merge (single-writer coordination)

`// TODO(s42): merge aesthetic prominence rows into mappings.json (coordinate with Music Theory — shared single-writer file)`

Under `composition.prominence_catalogue`, ADD:

```json
{ "id": "melody_forward", "layers": [
    { "role": "Melody",        "weight": 0.78 },
    { "role": "CounterMelody", "weight": 0.58 },
    { "role": "HarmonicFill",  "weight": 0.42 },
    { "role": "Pad",           "weight": 0.40 },
    { "role": "Bass",          "weight": 0.50 } ] }
```

Under `composition.prominence`, CHANGE `"default": "uniform"` → `"default": "melody_forward"`
and leave the existing `subject_melody` rule untouched (it remains the stronger,
true-subject tier).

Backward compatibility: `melody_forward` is a new catalogue id; old mappings.json still
parses (the field is an additive Vec row + a string default change); weights are all in
`[0,1]`; no schema change. The `uniform`/`subject_melody` profiles are unchanged, so any
external reference to them is preserved. The frozen `src/engine.rs` is not touched — the
planner already resolves `prominence` from this table and the realizer already consumes the
resolved weights; this slice only changes which profile a non-subject image resolves to.

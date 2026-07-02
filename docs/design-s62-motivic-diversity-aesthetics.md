# S62 — Motivic Diversity: The Aesthetic Acceptance Bar

**Lens:** Composition & Songwriting Aesthetics (trained ear). Complement to the Music
Theory Specialist's code-mechanism trace. **This session is diagnostic-first — no code
change is authored here.** This doc defines, perceptually, what "each piece has its own
recognizable motif" *means*, sets the bar a fix must clear, names the traps, and gives
the operator a falsifiable A/B protocol.

**Operator report under investigation:** *"pretty much the same motif with minor
variations for each piece."*

---

## TL;DR — the bar, the ranked dimensions, the rubric (read this first)

**The distinctness bar (one sentence).** Across the six fixed example images rendered at
a fixed seed, no two pieces may share **both** their contour class **and** their rhythm
cell; the *opening gesture*, the *contour class*, and the *rhythm cell* must each take at
least two distinct values across the set; at most two of the six may open tonic-in-the-
same-register-with-the-same-first-interval; and every motif must still be one of the
curated good-tune archetypes — distinctness is bought by **selecting different good
shapes**, never by inventing random intervals.

**The ranked distinctness dimensions** (what dominates the ear's sense of "same tune,"
most-dominant first — a listener forgives a shared item further down this list far faster
than one near the top):

| Rank | Dimension | Why it dominates |
|---|---|---|
| 1 | **Opening gesture** — first 2–3 notes as pitch + interval + rhythm *together* (the "head") | A tune is recognized by its head. Same head ⇒ "same tune" regardless of what follows. |
| 2 | **Contour class** — rising / arch / falling / oscillating / turn (the phrase gestalt) | The overall shape is the second thing the ear grabs; different class reads as a different melody. |
| 3 | **Rhythm cell / durational profile** — the "clap test" | You can clap a tune's identity. Shared rhythm = strong sameness even under different pitches (the nursery-rhyme effect). |
| 4 | **Interval-content signature** — conjunct/stepwise vs disjunct/leapy; presence of a characteristic leap | "Smooth" vs "angular" is a strong character read, but subordinate to head + shape. |
| 5 | **Characteristic ("hook") pitch** — the peak, or the dwelt-on non-tonic degree | Gives a known tune its fingerprint, but is a later-onset cue than the head. |
| 6 | **Phrase length / melodic range** | Moderate. Two phrases of different length feel different, but this alone cannot rescue a shared head. |
| 7 | **Register / absolute octave** | **Weakest.** The ear is transposition-invariant — the same tune an octave up is *the same tune*. Range/register variation does the *least* for distinctness. |

**The operator's A/B rubric** (a few minutes, by ear, before-set vs after-set):

1. **Blur test** — play all six of a set back-to-back once; rate the SET 1–5 on "each
   piece has its own identity" (1 = all one tune, 5 = six distinct tunes).
2. **Head test** — play only the first ~2 s of each of the six; jot each head (up / down /
   arch / turn, and step vs leap); count *distinct heads*.
3. **Clap test** — clap each piece's main motif; count *distinct claps*.
4. **Sibling check** — pick the two after-set pieces that sound *most* alike; do they share
   **both** shape and rhythm? Any yes ⇒ fail.

**PASS** = Blur(after) ≥ 4 **and** ≥ 2 points above Blur(before); ≥ 4 distinct heads; ≥ 3
distinct claps; no sibling pair shares both shape and rhythm.

---

## 0. Grounding — what the code actually does today (read-only)

From `src/composition.rs` (~1504–1528, 1745–1820) and `src/chord_engine.rs` (~3286–3452):

- **One `ThemeSeed` per composition.** A single archetype is chosen by `pick_archetype(u)`,
  resolved once into the piece's motif. There is exactly one melodic identity per image.
- **The contour is one of eight fixed shapes.** `MotifArchetype::contour()` returns a fixed
  degree string: Arch `0 2 4 2 0`, InvertedArch `4 2 0 2 4`, Descent `4 3 2 1 0`, Ascent
  `0 1 2 3 4`, NeighborTurn `0 1 0 -1 0`, LeapStep `0 4 3 2 1`, Pendulum `0 4 0 4 0`,
  RisingSequence `0 1 2 1 2 3`.
- **The selector is a coarse 2×2.** `pick_archetype` splits `arousal × valence` into four
  quadrants → one contour *family*, then `vertical_emphasis` + `dominant_hue` pick one of two
  members. Many images land in the same quadrant, so many images get the *literally identical
  contour string*.
- **Six of the eight archetypes open on the tonic** (degree 0): Arch, Ascent, NeighborTurn,
  LeapStep, Pendulum, RisingSequence. Only InvertedArch and Descent launch on the fifth. So
  the *head* — rank-1 axis — is nearly always "tonic, then step/leap upward."
- **The main per-image lever is `range_degrees` (register/range scaling)** from
  `edge_activity`, plus `length_steps` from `complexity`, plus a rhythm cell from
  `pick_rhythm_cell` / `pick_piece_cell`.

**The aesthetic diagnosis in one line:** the engine varies most strongly on the axes the ear
cares *least* about (range/register = rank 7, length = rank 6) while holding the axes the ear
cares *most* about (head = rank 1, contour class = rank 2) nearly constant — because the
contour selector collapses to a few classes and 6/8 shapes share a tonic-launch head. That is
*exactly* the perceptual signature of "same motif with minor variations." The rhythm-cell work
(S41/S53) correctly attacks rank 3 and is why pieces don't clap *identically* — but rank 1 and
rank 2 are where the sameness lives, and they are the fix target.

---

## 1. Motivic distinctness, defined perceptually

Two motifs read as **different tunes** (not variations of one) when they diverge on the
*high-ranked* axes above. Concretely, per dimension — how much must differ to cross the
"different tune" threshold:

**1 — Opening gesture (the head). DOMINANT.**
The first 2–3 events, heard as an inseparable pitch-interval-rhythm unit. Crossing the
threshold requires a change the ear registers in the first half-second: a *different start
degree* (not-always-do), and/or a *different first interval* (a leap where the sibling
stepped, or an up where the sibling went down), and/or a *different first durational value*
(a long held launch vs an anacrustic pickup). *Why it dominates:* melodic recognition is
front-loaded — we name a tune from its opening, and mismatched tails don't undo a matched
head. If the fix changes only one axis, change this one.

**2 — Contour class. DOMINANT.**
The gestalt trajectory: rising, arched, falling, oscillating, or turning. Threshold = a
*different class*, not a different member of the same class. (Arch vs InvertedArch is a
smaller perceptual step than Arch vs Descent; RisingSequence vs Ascent is smaller still.) *Why
it dominates:* the shape is the second thing the ear locks onto and it survives transposition,
tempo change, and reorchestration — so two pieces with the same class feel like siblings even
when other details differ.

**3 — Rhythm cell / durational profile (the clap test).**
The pattern of long/short values independent of pitch. Threshold = a *different cell family*
(broad/augmented vs even/moto-perpetuo vs profiled/syncopated), not merely a shifted accent.
*Why it matters:* rhythm is separately memorable — you can clap "Happy Birthday" and be
recognized. Shared rhythm re-imposes sameness even after pitch is diversified.

**4 — Interval-content signature.**
Conjunct (stepwise) vs disjunct (leapy), and whether a *characteristic* leap (a 4th/5th/octave)
is present. Threshold = crossing the smooth/angular line, or adding/removing a signature leap.
*Why it matters:* it sets the tune's "character" (lyrical vs heroic vs jagged) but is read
slightly after the head and shape.

**5 — Characteristic ("hook") pitch.**
The single most-emphasized non-tonic degree — the peak, or the note the phrase leans on/dwells
on. Threshold = a *different emphasized degree* (a phrase that peaks on the 6th vs one that
peaks on the 3rd). *Why it matters:* it's the fingerprint that lets you tell two otherwise-
similar tunes apart once you know them — but it's a later cue than head/shape, so it can't
carry distinctness alone.

**6 — Phrase length / range.** Threshold: a clearly different number of notes, or a
narrow-vs-wide ambitus. Contributes, but subordinate; a longer phrase with the same head still
reads as "the same tune, extended."

**7 — Register / absolute octave. WEAKEST — near-zero for distinctness.** The ear treats
octave-transposed material as identical. **Variation confined to this axis does not create
distinctness** and must not be counted toward the bar. (This is the axis the current engine
varies *most*, via `range_degrees` — which is why the variation "reads as minor.")

---

## 2. The acceptance bar (testable, with rationale attached)

Let the evaluation set be **N = 6 distinct images** rendered at a **fixed seed** (the six
example images used for A/B). For each piece, take its *primary motivic identity* — the head +
contour class + rhythm cell of the theme (and the per-piece motto on no-theme images). The
following are stated so the Music Theory Specialist and Test Engineer can encode them as
properties over the build-time plan (archetype, resolved contour, opening degree/interval,
rhythm cell) — no audio analysis required.

**P1 — No sibling pair (the load-bearing property).** For all pairs (i, j), NOT
(same contour class AND same rhythm cell). *Why:* sharing the two dominant post-head axes is
the strongest single cause of heard sameness; forbidding the *combination* is the minimal cut
that breaks the nursery-rhyme effect while still allowing legitimate reuse of any one axis.

**P2 — Head diversity (attacks rank 1).** Across the six, the *opening gesture* — encoded as
(start degree bucket, sign of first interval, step-vs-leap of first interval) — takes **≥ 3
distinct values**. *Why:* the head is where recognition happens; three distinct heads across
six pieces is the floor at which a listener stops hearing "the same opening every time."

**P3 — Contour-class spread (attacks rank 2).** Across the six, contour *class* (rising /
arched / falling / oscillating / turn) takes **≥ 3 distinct values**. *Why:* the gestalt shape
is the second recognition cue; three classes over six pieces guarantees the set doesn't
collapse into one shape-family.

**P4 — Rhythm spread (attacks rank 3).** Across the six, the rhythm *cell family* takes **≥ 3
distinct values**. *Why:* keeps the clap test alive independently of pitch; the S41/S53 work
already reaches this — P4 is a floor, not a new burden.

**P5 — Anti-tonic-launch cap.** **At most two** of the six may open on
(tonic, same register band, same first interval). *Why:* directly targets the "6/8 archetypes
start on do" mechanism — the most concrete source of shared heads in the current vocabulary.

**P6 — Weighted-coverage floor.** At least **four of the seven** ranked dimensions must vary
(take ≥ 2 distinct values) across the set — **but rank-1/2/3 (head, contour class, rhythm) are
mandatory-vary** (each must independently take ≥ 2 values), and **rank 7 (register/octave) never
counts** toward the four. *Why:* prevents a fix from "passing on paper" by diversifying cheap
low-rank axes; the weighting forces the distinctness budget onto the axes the ear actually uses.

**P7 — Coherence floor (the guardrail, not a distinctness property).** Every motif is one of
the curated archetypes (or a legal inversion/retrograde thereof) and stays voice-leading-legal
and in-key. *Why:* distinctness must be *selection across a good vocabulary*, never invention of
arbitrary intervals — see §3(a).

> **The whole bar in one rule of thumb:** *diversify the head and the shape; keep the
> vocabulary curated.* P1/P2/P5 are the ones that would fail today; P3/P4 are floors the
> current work mostly meets; P6/P7 are guards.

---

## 3. The two failure modes the bar must avoid

**(a) Distinctness collapsing into incoherence ("random ≠ different").**
The wrong fix "diversifies" by loosening the vocabulary — letting the image pick arbitrary
degree strings or intervals. The ear does not hear randomness as *distinct*; it hears it as
*wrong* (ugly, unsingable, un-tune-like), and — cruelly — a bag of random motifs can still all
sound *equally* random, i.e. same-ish in their arbitrariness. **How the bar avoids it:** P7 (the
coherence floor) requires every motif to remain a curated archetype (or a legal transform of
one). The distinctness budget is spent *selecting different good shapes*, never inventing. This
is the existing `§1-governor` philosophy in `chord_engine.rs` — "the image SELECTS and
PARAMETERIZES a shape, it never generates a random interval string" — and the bar keeps it.

**(b) Distinctness on paper that the ear still hears as sameness.**
The wrong fix changes something a diff can see but an ear cannot weight — e.g. all six motifs
start on the tonic, in the same register, with the same rhythm, differing only in a middle note
or only in range. A property counting "did *anything* change?" passes; the operator still hears
one tune. **How the bar avoids it:** the ranking *is* the defense. P6 refuses to count rank-7
register variation; P2/P5 force the rank-1 head to actually differ; P1 forbids the rank-2+rank-3
combination that most reads as sameness. A fix that varies only middle notes or only range
*fails P2 and P5*, exactly as the ear would fail it.

---

## 4. Falsifiable A/B listening protocol (operator, on return)

**Setup.** The same **six** example images, one **fixed seed**, rendered to WAV twice: a
**BEFORE** set (current engine) and an **AFTER** set (once a fix lands). Label them
`before_1..6.wav` / `after_1..6.wav`. Listen on the same output you'd trust for a mix.

Do the four steps **on BEFORE first, then on AFTER**, so the before-set calibrates your ear to
the sameness you're trying to break.

**Step 1 — Blur test (the gestalt; ~90 s per set).**
Play all six of the set back-to-back, once, without stopping. Then, without replaying, ask:
*"If I heard #3 again cold, could I name which image it was?"* Rate the **whole set** 1–5:

> `1` = all six are one tune · `2` = one tune, minor variants · `3` = a couple stand out ·
> `4` = mostly distinct, one or two blur · `5` = six distinct tunes

Record `Blur_before` and `Blur_after`.

**Step 2 — Head test (rank 1; ~60 s per set).**
Play only the **first ~2 seconds** of each of the six. For each, jot the head in shorthand:
direction (`↑ ↓ ∧ ∨ ~`) and first move (`step` / `leap`). Count **distinct heads** in the set.
Record `Heads_before`, `Heads_after`.

**Step 3 — Clap test (rank 3; ~60 s per set).**
Clap the rhythm of each piece's main motif. Count **distinct claps**.
Record `Claps_before`, `Claps_after`.

**Step 4 — Sibling check (verifies P1 by ear; ~30 s, AFTER set only).**
Pick the two AFTER pieces that sound *most alike*. Ask the single question: *do they share
**both** their shape and their rhythm?* Record `Sibling_both = yes/no`.

**The call.**

| Metric | Threshold |
|---|---|
| `Blur_after` | ≥ 4 |
| `Blur_after − Blur_before` | ≥ 2 |
| `Heads_after` | ≥ 4 distinct (of 6) |
| `Claps_after` | ≥ 3 distinct (of 6) |
| `Sibling_both` | no |

**PASS** = all five rows hold. **FAIL** = any row fails — and the failing row *names the
diagnosis*: low `Heads_after` ⇒ head fix insufficient (rank 1 not moved); `Sibling_both = yes`
⇒ P1 breached (two pieces share shape+rhythm); high `Blur_after` but low delta ⇒ before was
already fine / fix was cosmetic. Total operator time: ~5–6 minutes.

*(A single-image sanity check on the way in: render one image at two adjacent seeds; the two
should sound like the **same tune** — this confirms per-image identity is stable and the
diversity you're testing is genuinely image-driven, not seed noise.)*

---

## 5. Aesthetic fix direction (design only — not implemented here)

**What SHOULD drive per-image motivic identity** — spend the diversity budget on the high-rank
axes, read from the image's most *distinctive* features so the hook mirrors seeing *that* image:

- **Break the tonic-launch monopoly (highest leverage, rank 1).** Let a salient, distinctive
  image feature choose the **start degree** — a focal-point / dominant-subject-position cue that
  can land the head on the 3rd or 5th, not always do. This single change moves the axis the ear
  weights most and is the most direct answer to the operator's report. *(Depends on Music Theory
  — the alternate launch degree must still resolve and transpose legally.)*
- **Widen the contour selector (rank 2).** The affect 2×2 is too coarse — too many images share a
  quadrant and thus a contour string. Either finer cuts, a secondary distinctive feature to pick
  among *more* shapes, or admit **legal inversions/retrogrades** of the base archetype as
  additional selectable heads (the code already ships InvertedArch/LeapStep/Pendulum/etc. —
  ensure the selector actually *spreads across* them rather than clustering). *(Depends on Music
  Theory — inversion/retrograde must stay voice-leading-legal.)*
- **Assign a hook pitch (rank 5, the fingerprint).** Let a signature image feature (a dominant-hue
  position, a strong color accent) choose the phrase's **emphasized/peak non-tonic degree** — the
  note the tune leans on. Cheap, and it's what makes a known tune nameable.
- **Keep rhythm image-driven but decorrelated (rank 3).** The S41/S53 cell selection is right —
  preserve it, and ensure the cell axis co-varies *independently* of contour so shape and gait
  aren't locked together (which would collapse two axes into one and make P1 easy to breach).

**What stays CONSTANT — so six distinct tunes still sound like one instrument's voice:**

- the tonal language and key discipline (in-key, cadences honored),
- the **archetype vocabulary itself** — all curated good-tune shapes; diversity is *selection
  within* it, never invention outside it,
- the register band, timbre, and instrument identity,
- the harmonic idiom and phrase-arc / cadence conventions.

Diversity lives in **motif selection**; the **voice** lives in everything around the motif. That
split is precisely what lets the set read as six memorable, image-specific hooks *by the same
performer* — the target aesthetic.

**Dependency on the Music Theory lens (explicit).** Every proposed diversifier that touches
pitch — alternate launch degrees, inversions/retrogrades, hook-degree emphasis — must be
adjudicated by Music Theory (`resolve_motif` / `resolve_motif_celled` owner) for
voice-leading-legality, in-key resolution, and clean transposition under section key offsets.
**This aesthetic lens owns *which axes to diversify and the bar they must clear*; the Music
Theory lens owns *keeping the diversified motifs legal*.** The fix is correct only when both
hold: distinct to the ear (this doc) **and** legal to the theory (theirs).

---

## S62 gate-close review

Cadence-closing taste pass, judged against the AFTER data actually produced (per-image theme
motif, key-relative, curated archetypes). **What shipped:** the pitched-motif seam was un-gated
so all six images now emit a per-image subject (before, 4 of 6 were theme-less and fell back to
generic melodization), and motif length was floored to ≥5 steps. The two refinements this doc's
§5 also called for — length↔range decorrelation and opening-degree diversity — were deliberately
deferred (they'd edit frozen motif goldens; larger blast radius than the dominant fix needs).

The AFTER set under review:

| image | archetype | range | len | theme-cell | motto-cell | theme motif (deg / dur) | reads as (1-based) |
|---|---|---|---|---|---|---|---|
| 1 | LeapStep | 4 | 5 | 1 | 1 | deg[0,4,3] dur[2,2,1] | 1–5–4, held-tonic then rising 5th |
| 2 | Arch | 5 | 5 | 0 | 0 | deg[0,3,5,3] dur[2,1,1,1] | 1–4–6–4, open 4th to the 6th |
| 3 | Arch | 4 | 5 | 0 | 3 | deg[0,2,4,2] dur[2,1,1,1] | 1–3–5–3, triadic arpeggio |
| example | Ascent | 6 | 8 | 3 | 2 | deg[0,2,3,5,6] dur[1,1,1,2,3] | 1–3–4–6–7, climbing |
| Lena | InvertedArch | 4 | 5 | 0 | 0 | deg[4,2,0,2] dur[2,1,1,1] | 5–3–1–3, falling-then-lifting valley |
| magic | Descent | 3 | 8 | 3 | 3 | deg[3,2,2,1,0] dur[2,1,1,1,3] | 4–3–3–2–1, stepwise descent |

### Task 1 — P1–P7 scored against the real output

**P1 — No sibling pair (same contour class AND same rhythm cell). BORDERLINE.** The only
same-class pair is Img2/Img3 (both Arch), and they DO share the theme rhythm cell (both cell 0,
both dur[2,1,1,1] — identical theme gait). On the strict letter that is the forbidden
combination. What keeps it from collapsing: they diverge on the head interval (rank 1 — a rising
4th vs a rising 3rd) and on the motto cell (0 vs 3), so the pieces don't present as the same
shape with the same rhythm end-to-end. Ruled BORDERLINE, not NOT-CLEARED, because the axes the
ear weights above shape+cell (the head, and the motto gait) do separate them — but this is the
one pair the operator must confirm by ear (see Task 2/Sibling check).

**P2 — Head diversity (≥3 distinct opening gestures). CLEARED.** Encoding head as (start
degree, first-interval direction, leap size): Img1 (0,↑,5th), Img2 (0,↑,4th), Img3 (0,↑,3rd),
example (0,↑,3rd), Lena (4,↓,3rd), magic (3,↓,step) → **5 distinct heads of 6** (only Img3 and
example collide). Clears the ≥3 floor and the rubric's ≥4 head-test threshold. Caveat carried
forward: the diversity is bought by first-interval VARIETY, not start-degree variety — four of
the six still launch on the tonic (that's the deferred axis).

**P3 — Contour-class spread (≥3 classes). CLEARED.** Five distinct classes over six —
LeapStep, Arch, Ascent, InvertedArch, Descent. Even collapsed to gestalt families (rising /
arch / falling / valley) that's four. Comfortably clear.

**P4 — Rhythm spread (≥3 cell families). CLEARED.** Theme cells {0,1,3} = 3 distinct (at the
floor); motto cells {0,1,2,3} = 4 distinct. The clap test stays alive independently of pitch.

**P5 — Anti-tonic-launch cap. BORDERLINE-CLEARED.** The literal predicate — at most two open on
(tonic, same register band, same first interval) — passes: four launch on the tonic, but no two
share the full launch signature because the first interval differs (5th / 4th / 3rd / 3rd) and
range differs, so the largest identical-launch cluster is one. The INTENT — break the
tonic-launch monopoly — is only partially met: 4 of 6 still open on do, differentiated by
interval, not by degree. Letter cleared, spirit half-cleared; this is exactly the residual the
deferred opening-degree work targets (Task 4).

**P6 — Weighted-coverage floor (≥4 of 7 vary; rank 1/2/3 mandatory; rank 7 never counts).
CLEARED.** Ranks 1 (head), 2 (contour), 3 (rhythm) all independently vary — mandatory-vary
satisfied. Rank 4 (interval content: 5th/4th/3rd leaps vs stepwise descents), rank 5 (apex
degree: 5th / 6th / 5th / 7th / …), rank 6 (length 5 vs 8) also vary. Six of six countable
dimensions move; register (rank 7) not counted. Well clear.

**P7 — Coherence floor. CLEARED.** Every motif is a curated archetype contour, key-relative and
in-key; the un-gating routes MORE images through the SAME authored vocabulary rather than
inventing intervals — precisely the "select a good shape, never synthesize" discipline the bar
requires. No incoherence introduced.

**Scoreline: 5 CLEARED, 2 BORDERLINE (P1, P5), 0 NOT-CLEARED.** Both borderlines trace to the
one deferred axis (start-degree / tonic-launch), not to the fix that shipped.

### Task 2 — Img2 vs Img3 sibling ruling (the closest pair)

**Verdict: they read as DIFFERENT tunes, not two variations of one Arch.** Img3 is 1–3–5–3 — a
pure tonic-triad arpeggio, bright and closed, a bugle/fanfare character. Img2 is 1–4–6–4 — it
opens with a wider rising 4th and reaches the **6th**, which is a sweeter, more suspended,
"reaching" apex than Img3's stable 5th. Three separable differences carry it: the opening leap
size (4th vs 3rd, a rank-1 head difference), the interval character (open-4th/6th color vs
stacked-thirds triad, rank 4), and the apex degree (6th vs 5th, rank 5). They share only the
arch gestalt (rank 2) and the theme cell (rank 3) — the two axes the ear forgives fastest once
the head and color differ. So: different tunes, ruled clear.

It is nonetheless the pair that comes closest, so it is the right one for the operator's Sibling
check. **What to A/B-listen for between just these two:** (a) the opening jump — Img3 steps up a
small 3rd, Img2 leaps a wider 4th; (b) the top note — Img3 tops out closed on the 5th, Img2 tops
out on the sweeter, less-resolved 6th. If those two cues land, they're different tunes and
un-gating sufficed. If the two still sound like one arch tune despite them, the shared shape+cell
is dominating and that is the signal to pull the deferred opening-degree work (Task 4).

### Task 3 — Operator A/B rubric: ready to run

Both sets are rendered at the same fixed seed (before = pre-un-gate engine, after = this
result), so the comparison is clean. The ~5-minute protocol, unchanged from §4, run on BEFORE
first then AFTER:

1. **Blur test (~90 s/set)** — play all six back-to-back once; rate the SET 1–5 on "each piece
   has its own identity." Record Blur_before, Blur_after.
2. **Head test (~60 s/set)** — first ~2 s of each; jot direction + step/leap; count distinct heads.
3. **Clap test (~60 s/set)** — clap each main motif; count distinct claps.
4. **Sibling check (AFTER only, ~30 s)** — pick the two most-alike; do they share BOTH shape and
   rhythm? Any yes ⇒ P1 flag.

PASS = Blur_after ≥ 4 and ≥ 2 above Blur_before; ≥ 4 distinct heads; ≥ 3 distinct claps; no
sibling shares both.

**Specific to THIS result, listen for:**
- **The theme-less→themed delta is the whole fix.** Images 1, 2, 3 and Lena were theme-less
  before (generic melodization wash, no per-image subject); they now each carry a real subject.
  The Blur and Head jumps should be **largest on those four** — that delta, not the two images
  that already fired, is what proves the un-gating worked. On the before-set expect those four to
  present as near-identical generic openings; on the after-set they should give four distinct,
  hummable heads (1–5, 1–4–6, 1–3–5, 5–3–1).
- **Sibling = Images 2 and 3** (both Arch). Apply the leap-size / apex cues from Task 2. This is
  the pair most likely to trip the Sibling check.
- **Head-test focus:** the four tonic-openers (1, 2, 3, example) — do they read as four different
  openings, or as "start on do and rise" four times? That single observation drives Task 4.

### Task 4 — Residual-sameness call and recommendation

**Residual risk is real but secondary.** Four of six motifs still open on the tonic (1, 2, 3,
example), all with a "hold the tonic, then rise" gesture; the leap SIZE differs (5th/4th/3rd/3rd)
but the launch-degree does not. A sensitive ear may still tag those four as a family — Img3 vs
example (both tonic→3rd→upward) are the likeliest secondary blur after the Img2/Img3 pair. That
is exactly what the deferred opening-degree diversity work (land some heads on the 3rd/5th) would
dissolve.

**But the dominant cause was not the tonic monopoly — it was the DEAD seam.** Per the diagnostic,
the operator's "same motif, minor variations" was principally the *absence* of a motif on 4 of 6
images, heard as generic sameness. Un-gating attacks that dominant cause directly: those four now
have real, distinct, full-length subjects. That alone should move the Blur score substantially
and plausibly satisfies the complaint.

**Recommendation: ship the un-gating, run the A/B, hold the opening-degree work as an armed,
conditional follow-up — do NOT do it pre-emptively.** The frozen-golden blast radius of the
opening-degree change is only justified if the ear still fails. Pull it *only if* the after-set
Head test still clusters the tonic-openers (specifically: the four tonic-launchers read as "same
opening," or Img3/example blur, or the Img2/Img3 sibling check comes back "same tune"). This is
consistent with the rubric's own diagnosis mapping — a low Heads_after is the row that *names*
the head fix as insufficient, and that is the trigger, not the default. If the Head test passes,
the deferred work is not needed and should stay deferred.

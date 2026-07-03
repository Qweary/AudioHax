# S63 — Holistic Per-Piece Distinctness: The Aesthetic North Star

**Lens:** Composition & Songwriting Aesthetics (trained ear). Complement to the Music
Theory Specialist's mechanical vocabulary inventory (library sizes + per-image
utilization). **This session is diagnostic/design-first — no code is authored here.**
This doc defines, perceptually, what makes a set of pieces feel like *distinct, integrated,
memorable wholes* rather than "the same thing with variations," sets the higher-order
targets a mechanical fix must serve, names the anti-patterns as design law, and gives an
ear-checkable acceptance bar for a multi-session build arc.

**Operator report under investigation (verbatim):**
- *"the rhythms sound repetitive (the tempo is varied)"*
- *"the overall feel of the accompaniment and melody feels like it's the same thing with variations"*
- *"The 'motif' or 'theme' is a combination of all factors in my head"* — melody + accompaniment + rhythm + how they integrate, not just pitch.
- North star: *"What is truly unique about music is how every part of it works together, but has its own part to it, every piece has unique takes and aspects despite following familiar rules, and every distinct piece has some sort of surprise to it that sets it apart from any other."*
- Balance: *"I know there is a balance between what we have vs randomness, but I think we can do better."*

**What changed since S62:** the pitch motif was un-gated, so each image now yields
*different notes*. The operator confirms that helped. The residual complaint is no longer
"same pitches" — it is **same gestalt**: same rhythmic feel, same accompaniment/melody
texture, no per-piece surprise. S62 fixed the *melodic subject*; S63 targets the *whole
cloth the subject is woven into*.

---

## TL;DR — read this first

**The residual sameness is a TEXTURE-and-RHYTHM gestalt problem, not a pitch problem.**
Pitch now varies per image; what still reads as "one piece with variations" is that **every
role is chosen by an independent lookup and then stacked**, and the **rhythmic surface is
narrow and shared**. Three findings drive it, ranked:

1. **[DOMINANT] The parts are STACKED, not INTEGRATED.** Melody, accompaniment figuration,
   and bass are each selected by a separate first-match-wins table from a separate catalogue.
   Nothing makes the accompaniment *respond* to the melody, and nothing makes the bass and
   the comp *complement* each other rhythmically. The one shared per-piece rhythmic DNA (the
   motto) biases only the melody's onsets — it does not reach the accompaniment or bass. The
   ear hears three correct-but-indifferent layers running in parallel: the signature of
   "templates stacked together."
2. **[HIGH] The rhythmic surface is narrow and cross-piece-shared.** Accompaniment figuration
   is a small static onset table; the bass default is one sustained root per step. Tempo
   varies (the operator noticed), but tempo is not rhythm — the *pattern of attacks* barely
   moves across pieces, so every piece claps the same even at different speeds.
3. **[MEDIUM] There is no per-piece SURPRISE.** Every piece is "well-behaved all the way
   through." Nothing is set aside to be the one memorable event — the turn, the break, the
   silence — that makes *this* piece nameable. Familiar-rules-without-a-surprise is precisely
   "pleasant but interchangeable."

**INTEGRATION** (defined below) = the parts share one per-piece rhythmic/gestural DNA and
then *differentiate their roles against it* (complementary, not unison; responsive, not
indifferent). **CONTROLLED SURPRISE** = a feature-seeded, legal-bounded, deterministic
departure placed at one curated moment — never a dice roll. **The holistic bar** (§4) gates
a set on rhythm-feel spread, texture identity, integration, and a nameable surprise per piece.

**Recommended FIRST front (highest leverage for the ear): INTEGRATION via a shared per-piece
rhythmic DNA that propagates into the accompaniment and bass.** It is the single change that
converts three stacked layers into one interlocking cloth, and it simultaneously attacks
finding #1 (stacking) and finding #2 (narrow shared rhythm), because a per-piece DNA that
reaches all roles is *both* the interlock mechanism *and* the source of cross-piece rhythmic
variety. Breadth and surprise are amplifiers that land better once the cloth is woven.

---

## 0. Grounding — what the engine actually does today (read-only)

From `src/composition.rs` (the planner) and its catalogues in `assets/mappings.json`:

- **Every role is an INDEPENDENT first-match-wins selection.** `plan()` calls `.select(u)`
  separately for `form`, `character`, `meter`, `key_scheme`, `theme_behaviour`, `texture`
  (the `OrchestrationProfile`), then resolves `figuration`, `bass_pattern`, and `prominence`
  handles — each against its *own* catalogue, each keyed off image knobs *independently*.
  There is no cross-role constraint: the accompaniment figuration does not know what the
  melody's rhythm is; the bass does not know what the figuration is doing.
- **The one shared per-piece rhythmic element reaches only the melody.** The `RhythmMotto`
  (S53 D-CELL) is computed once per piece from `(edge_activity, complexity)` and stamped on
  every section — a genuine per-piece rhythmic DNA. But the realizer reads it *only* to bias
  the melody's onset placement. The accompaniment (`FigurationSpec`, a static onset table)
  and the bass (`BassPatternSpec`, default `Sustained` = one root per step) are unaffected by
  it. The DNA exists but is not expressed by the other parts.
- **The accompaniment vocabulary is a small static onset table.** `FigurationSpec.onsets` is
  2–4 fixed fractional positions within a step. A figured profile runs its cell as an
  ostinato (S45 adds a departure-and-return arc by thematic role, but within the *same* small
  catalogue). The bass default holds one root per step. So the rhythmic *surface* — the
  pattern of attacks the ear claps — is shallow and mostly shared across pieces.
- **Tempo varies per image** (`brightness → BPM`, per-character window), which is why the
  operator hears the tempo move. But two pieces at different tempi with the same attack
  pattern and the same layer relationship still read as the same piece played faster/slower.

**The aesthetic diagnosis in one line:** the engine selects each part correctly and
independently, then plays them at the same time — so the pieces differ in their *ingredients*
but not in *how the ingredients are cooked together*, and the ear weights the cooking (the
integrated rhythmic-textural gestalt) far above the ingredient list.

---

## 1. Diagnosis — the residual gestalt sameness, ranked

The dimensions most responsible for the *whole-piece* sameness a trained ear hears, most
dominant first. (This ranking is the design defense in §4: the bar must force variation onto
the high-rank dimensions and refuse to be satisfied by low-rank ones.)

### Rank 1 — STACKED (non-interlocking) texture. DOMINANT.
*What it is.* The layers are chosen independently and sound simultaneously with no mutual
constraint. Two pieces can have different melodies, a different figuration cell, and a
different bass pattern, yet still feel like "the same arrangement" because the *relationship*
between the layers is identical: melody-on-top, comp-in-the-middle, bass-underneath, each
minding its own business, none reacting to the others.
*Why it reads as sameness.* The ear identifies an arrangement by its *interplay* — call and
response, the way the comp fills the melody's gaps, whether the bass walks toward the next
chord or sits. That interplay is a fixed template here, so every piece presents the *same
conversation* with different words. A listener registers "same kind of thing" from the
interplay before they finish parsing the notes. This is the perceptual core of the operator's
"accompaniment and melody feels like the same thing with variations."

### Rank 2 — Narrow, cross-piece-shared RHYTHMIC SURFACE. HIGH.
*What it is.* The attack pattern of the accompaniment is a small static table; the bass is
usually one root per step; the melody's gait is one of a few cells. Tempo changes, but the
*pattern* of long/short/where-the-hits-land barely changes piece to piece.
*Why it reads as sameness.* Rhythm is separately, powerfully memorable — you can clap a tune's
identity. If every piece's clap-pattern is drawn from the same shallow pool, the pieces re-
converge rhythmically even after pitch is diversified. The operator's "rhythms sound
repetitive (the tempo is varied)" is exactly this: tempo ≠ rhythm, and the rhythm is what's
shared. This is also *within*-piece: a 2–4-onset figuration run as an ostinato for ~32 bars
loops audibly.

### Rank 3 — No per-piece SURPRISE. MEDIUM-HIGH.
*What it is.* Every piece follows the familiar rules cleanly from start to end. Nothing is
withheld and then spent as a single memorable event.
*Why it reads as sameness.* Memorability attaches to the *exception*, not the rule. A piece
that never deviates has nothing for the ear to file it under; a set of such pieces is a set of
equally-well-behaved, equally-forgettable objects — "familiar rules" with no "surprise that
sets it apart." This is the direct target of the operator's "every distinct piece has some
sort of surprise to it."

### Rank 4 — Uniform DENSITY / textural arc. MEDIUM.
*What it is.* Within a piece the density is roughly constant (S45's role-based figuration arc
helps a little; density is largely flat otherwise). Pieces don't *breathe* — no build, no
thinning, no arrival.
*Why it reads as sameness.* A shared dynamic/density contour makes two pieces feel like the
same emotional journey. Distinct pieces rise and recede in their own places.

### Rank 5 — Shared registral/voicing layout. LOWER.
*What it is.* Roles sit in fixed registral bands with a fixed spacing.
*Why it matters, but less.* Voicing color is real but the ear weights it below rhythm and
interplay; two pieces with the same layout but different interplay already read as different.

> **The rule of thumb:** the ear ranks *how the parts move together* (rank 1–2) and *whether
> the piece has one memorable exception* (rank 3) far above the ingredient list. Today's
> engine varies ingredients (pitch, tempo, which cell) while holding the interplay, the
> rhythmic surface, and the surprise-budget constant — the exact inversion of what the ear
> weights, which is why the variation "reads as minor."

---

## 2. INTEGRATION, defined concretely (the operator's core value)

Operator's value: *"every part works together but has its own part to it."* In craft terms
this is the difference between a **texture** (parts derived as one interlocking whole) and a
**stack** (parts sampled independently and overlaid). The failure today is the stack. Here are
the concrete, buildable properties of an integrated texture — each stated as a design target,
not a vibe.

**I1 — One per-piece rhythmic DNA, expressed differently by each role.** There is already a
per-piece `RhythmMotto`. Integration means *every* role derives its rhythm from that one DNA,
but *renders it in its own idiom* — not in unison. Concretely: if the piece's DNA is
"long–short–short," the melody states it as pitched onsets, the comp *answers it in the gaps*
(fills where the melody holds), and the bass *anchors its strong pulse* (attacks the DNA's
downbeats, rests on its offbeats). Same DNA, three complementary expressions. *Why more
whole:* shared DNA is what makes a piece sound *composed* rather than *assembled* — the parts
are recognizably from the same idea. *Why not sameness across pieces:* because the DNA is
per-image, a different image yields a different DNA that all three roles then re-express — so
integration is *also* a cross-piece diversity multiplier (one lever moves all three layers'
rhythm at once).

**I2 — Complementary (interlocking) rhythm between roles, not parallel.** The roles must not
all attack together. The design target is *rhythmic complementarity*: the union of the roles'
onsets is denser and more even than any single role, because they fill each other's gaps
(the melody breathes while the comp moves, and vice-versa). Concretely, the accompaniment's
onset table should be *derived as the complement* of the melody's motto gait for that piece
(place comp onsets where the melody rests), rather than chosen from a catalogue that knows
nothing about the melody. *Why more pleasing:* interlock is the difference between an ensemble
and a pile-up; it is what "groove" and "counterpoint" both are at root. *Balance held:* the
complement is *computed deterministically from the melody's gait*, so it is fully structured —
zero randomness. It is more variety with *less* entropy, because it is derived, not sampled.

**I3 — Accompaniment figuration that RESPONDS to the melody's activity.** When the melody is
active (many onsets, wide contour), the comp should recede (thin, sustain); when the melody
holds or rests, the comp should animate (fill, arpeggiate). Concretely: modulate the resolved
figuration's onset density by an inverse function of the melody's local onset density for that
section. *Why more whole:* this is the single most recognizable marker of *arrangement* as
opposed to *layering* — a listener hears the parts "listening to each other." *Balance held:*
the response is a deterministic function of the (already per-image) melody, not a new random
knob.

**I4 — Bass that participates in the harmonic motion, matched to the piece's energy.** The
`Walking` and `Pedal` bass kinds already exist but are rarely selected. Integration means the
bass *kind* is chosen to complement the comp and the harmonic rhythm: a busy comp over slow
harmony wants a pedal or sustained bass (don't crowd); a sparse comp over moving harmony wants
a walking bass (carry the motion). Concretely: select `bass_pattern` as a function of the
resolved figuration's density and the harmonic rhythm, not from an independent table. *Why more
pleasing:* the low end either grounds or propels — matched to what's above it, it completes the
texture; mismatched or indifferent, it muddies. *Balance held:* a lookup keyed on *other
resolved roles*, still deterministic.

**I5 — A shared per-piece gestural signature across pitch and rhythm.** The strongest
integration ties the *pitch* motif and the *rhythmic* DNA to the *same* per-image seed, so the
piece's melodic shape and its groove feel like two views of one idea (e.g. an image that reads
"angular/energetic" gets both a leaping contour *and* a syncopated DNA; a "calm/flowing" image
gets a stepwise contour *and* a broad, even DNA). Today the archetype and the motto cell are
both derived from the image but through partly independent paths; the target is to make their
*character* co-vary intentionally. *Why more whole:* it is the difference between a piece that
has a *personality* and one that has a melody and a beat that happen to coexist. *Balance held:*
co-variation is a constraint that *removes* degrees of freedom — the opposite of randomness.

**The integration test (one sentence):** if you muted the melody, could you still tell which
piece it was from the accompaniment-and-bass alone? Today: no (the comp/bass are near-generic).
After: yes (the comp/bass carry the piece's DNA). That is the buildable definition of "every
part has its own part to it."

---

## 3. CONTROLLED SURPRISE, defined as bounded deviation (never randomness)

The operator wants *"some sort of surprise that sets each piece apart"* **and** explicitly
wants the structure-vs-randomness balance held. These are not in tension if surprise is
designed as a **feature-seeded, legal-bounded, deterministic departure** — a *curated
exception*, not a dice roll.

**The four laws of a legitimate surprise:**

**S-LAW 1 — SEEDED by the image, not by chance.** The surprise's *presence, location, and
kind* are a deterministic function of the image's features (e.g. the position of the salient
subject picks *where* in the form the surprise lands; a strong color accent or a bimodal
palette picks *which kind*). Same image → same surprise every render. *Why:* reproducibility is
what separates a signature from noise; a surprise you can't reproduce is a bug, not a feature.

**S-LAW 2 — BOUNDED to the legal vocabulary.** A surprise is a *departure within the rules*,
not a break of them. It must stay in-key (or make a *legal, resolved* excursion), voice-
leading-legal, and inside the curated shape/cell/figuration vocabularies. The surprise is
"unexpected *given the piece's own pattern*," not "unexpected given music." *Why:* the ear
hears an in-bounds deviation as *wit* and an out-of-bounds one as *error*; only the former is
memorable-in-a-good-way.

**S-LAW 3 — CURATED and SINGULAR.** One surprise per piece (or at most a small, fixed budget),
placed at a structural hinge (a phrase end, the final cadence, the top of the form). Surprise
spent everywhere is just noise and re-collapses to sameness (a bag of random events all sounds
equally random — see §3 anti-pattern). *Why:* memorability is scarcity; the exception only
reads as an exception against an otherwise-consistent background.

**S-LAW 4 — WITHHELD then SPENT.** The piece should establish its pattern first, so the
deviation has something to deviate *from*. A surprise in bar 1 is just the pattern. *Why:* the
setup is what makes the payoff land; this is the oldest rule in songwriting.

**Where a surprise can legitimately live (a menu, each seeded + bounded):**

| Locus | The deviation | Kept bounded by |
|---|---|---|
| **Harmonic turn** | one borrowed/chromatic chord or a deceptive cadence at a phrase end | drawn from the existing legal chord vocabulary; resolves; single occurrence |
| **Registral break** | the melody's peak jumps an octave, or one phrase drops to a low register | seated in the safe band, one phrase only, returns |
| **Textural shift** | one section strips to bare melody, or the comp drops out under the hook | uses existing density/figuration levers; one section only |
| **Rhythmic hemiola / displacement** | one phrase groups the DNA in 3s against a duple meter, or displaces the accent | derived from the piece's own DNA (a legal regrouping, not a new cell); resolves back |
| **Silence / caesura** | a beat of rest before the final statement | a single placed rest; the strongest, cheapest surprise |
| **Timbral/role flip** | the counter-melody takes the tune for one phrase | uses existing roles; returns |

**Design law — the anti-pattern, stated explicitly and forbidden:**

> **RANDOMNESS THAT BREAKS COHERENCE IS NOT SURPRISE.** A per-onset or per-note random
> perturbation, an unseeded RNG-chosen interval, a probability of "doing something weird" — all
> forbidden. They fail on three counts: (1) not reproducible (violates S-LAW 1), (2) tend to
> exit the legal vocabulary (violate S-LAW 2), and (3) *sound the same as each other* — a set
> of randomly-perturbed pieces all shares the quality "randomly perturbed," which is itself a
> sameness. The distinctness budget is spent *placing one curated exception*, never *scattering
> entropy*. If a proposed surprise cannot state its seed (which feature), its bound (which legal
> vocabulary), and its single locus, it is not admissible.

**Why this holds the operator's balance:** every legitimate surprise *reduces* apparent
entropy relative to true randomness — it is one deterministic, in-bounds event at a chosen
place. It buys *more* felt distinctness with *less* actual randomness than the current engine's
independent-sampling already spends. That is precisely "we can do better" on the
structure-vs-randomness trade: better = more curation, not more dice.

---

## 4. The holistic acceptance bar (ear-checkable, an evolution of the S62 A/B rubric)

The S62 rubric gated *melodic* distinctness (head / contour / clap). S63 widens it to the
*whole piece*: rhythm-feel, accompaniment/texture identity, integration, and a nameable
surprise. Evaluation set: the **six** example images, **fixed seed**, rendered **BEFORE** and
**AFTER** a fix. Some criteria are ear-only (operator); some are also encodable as build-time
properties over the plan (for the Music Theory / Test Engineer lenses) — flagged `[ear]` /
`[prop]`.

**H1 — Texture-identity test `[ear]`.** *Mute the melody; play the accompaniment+bass of all
six back-to-back.* Rate the SET 1–5 on "each accompaniment has its own identity." **PASS ≥ 4.**
*Why:* directly measures rank-1 stacking — if the comp/bass are generic, they blur with the
melody muted. This is the criterion today would fail hardest.

**H2 — Clap-the-arrangement test `[ear]`.** *Clap the combined rhythmic surface (melody +
comp + bass hits) of each piece.* Count **distinct claps**. **PASS ≥ 4 of 6.** *Why:* measures
rank-2 rhythmic-surface variety at the *arrangement* level, not just the melody (the S62 clap
test was melody-only). Catches "tempo varied but rhythm shared."

**H3 — Interlock test `[ear]` `[prop]`.** For each piece, ask: *do the parts fill each other's
gaps (interlocking), or do they attack together (parallel)?* `[prop]`: the union of role onsets
per section is denser and more even than any single role's (a computable complementarity
metric). **PASS:** ≥ 5 of 6 read as interlocking, not parallel. *Why:* this is the buildable
proxy for "every part has its own part" (I2/I3).

**H4 — Name-the-surprise test `[ear]`.** For each piece, the operator must be able to **name
one surprise in a short phrase** ("the octave jump before the end," "the bar of silence,"
"the deceptive cadence," "the hemiola in the B section"). **PASS:** a nameable surprise exists
for ≥ 5 of 6, and **no two pieces share the same surprise kind+locus** (else the "surprise" is
itself a template). *Why:* operationalizes rank-3; the "no two share" clause prevents surprise
from becoming a new sameness.

**H5 — Blur / whole-piece distinctness test `[ear]`.** *Play all six full pieces back-to-back
once; rate the SET 1–5 on "each is a distinct, integrated whole."* **PASS: Blur_after ≥ 4 AND
≥ 2 above Blur_before.** *Why:* the top-level gestalt judgment the whole arc serves; the delta
guards against "was already fine / fix was cosmetic."

**H6 — Sibling check `[ear]`.** *Pick the two AFTER pieces that sound most alike.* Do they
share **their interplay/texture-feel AND their rhythmic surface AND their surprise**? Any
double-or-triple overlap ⇒ investigate. **PASS: no sibling pair overlaps on ≥ 2 of {texture-
feel, rhythmic surface, surprise}.** *Why:* the load-bearing "did we actually break the
gestalt sameness" check, widened from S62's shape+rhythm pair to the whole-cloth axes.

**H7 — Coherence floor `[prop]` (guardrail, not a distinctness property).** Every piece stays
in-key, voice-leading-legal, within the curated form/contour/cell/figuration/bass vocabularies;
every surprise is seeded + bounded per §3. **PASS: no incoherence, no unseeded randomness, no
out-of-vocabulary material introduced.** *Why:* distinctness must be *selection and derivation
across a good vocabulary plus one curated exception* — never invention or entropy. This is the
non-negotiable that keeps the operator's balance.

**H8 — Per-image stability sanity `[ear]`.** Render one image at two adjacent seeds; the two
must sound like the **same piece** (same DNA, same surprise, same interplay). *Why:* confirms
the distinctness is *image-driven and deterministic*, not seed noise — the operational proof
that S-LAW 1 holds.

> **The whole bar in one rule of thumb:** *with the melody muted the pieces still differ (H1);
> the arrangement claps differently (H2); the parts interlock (H3); each has one nameable,
> non-shared surprise (H4); the set reads as distinct wholes (H5/H6); nothing left the
> vocabulary and nothing is random (H7); and it's all deterministic per image (H8).* H1/H2/H3
> are what today fails; H4 is the new axis; H7/H8 are the balance guards.

---

## 5. Sequencing the fronts — build order by leverage-per-effort for a trained ear

Three fronts are on the table: **breadth** (bigger curated libraries — the Music Theory pass's
domain), **integration** (interlock — §2), **surprise** (bounded deviation — §3). Ordered by
how much each moves the operator's verdict per unit of effort:

### FIRST — INTEGRATION (propagate the per-piece rhythmic DNA into accompaniment + bass).
**Highest leverage. Do this first.** It is the only front that attacks the *dominant* cause
(rank-1 stacking) *and* the *second* cause (rank-2 shared rhythmic surface) with one mechanism:
a per-piece DNA that reaches all roles is simultaneously (a) the interlock that turns a stack
into a texture and (b) a cross-piece variety multiplier (one image-driven DNA now moves three
layers' rhythm, not one). The machinery is *mostly already present* — the `RhythmMotto` exists
and is per-piece; the work is to let the accompaniment and bass *express* it (I1–I4) instead of
being sampled independently. Leverage-per-effort is highest because the carrier already exists;
this is a *wiring* change (who reads the DNA), not new material. **This is the single front
that, fixed first, moves the operator's verdict the most** — because "the accompaniment and
melody feels like the same thing with variations" is precisely a stacking complaint, and
interlock is its direct answer.

*Concrete first slice:* derive the accompaniment figuration's onsets as the **complement** of
the piece's motto gait (comp fills where the melody rests — I2), and modulate its density
**inversely** to the melody's local activity (I3). Ship it, run H1/H2/H3 by ear.

### SECOND — SURPRISE (one seeded, bounded departure per piece).
**Second-highest leverage, low effort.** Once the pieces are integrated wholes, a single
curated exception per piece is what makes each *nameable* — it converts "distinct" into
"memorable." It is cheap (one placed event, §3 menu) and high-impact because memorability
attaches to the exception. It comes *after* integration because a surprise needs an integrated,
consistent background to deviate *from* (S-LAW 4) — a surprise inside a stack is just more
stack. Start with the cheapest, strongest locus: **a placed silence/caesura before the final
statement**, seeded by a robust image feature, then add the harmonic turn and the registral
break as the vocabulary grows.

### THIRD — BREADTH (bigger curated figuration / bass / cell / contour libraries).
**Amplifier, not a fix — do it last (and continuously).** Breadth is the Music Theory pass's
domain and it *matters* — a shared narrow rhythmic surface (rank 2) is partly a library-size
problem. But breadth without integration just gives you a *bigger stack*: more ingredients,
same cooking, and the ear still hears the templated interplay. Breadth pays off *most* once
integration is in place, because then each new figuration/bass/cell is expressed *through* the
per-piece DNA and interlock, so a larger library translates into more felt distinctness rather
than more independent samples. Grow the libraries steadily underneath the integration and
surprise machinery; it lifts the ceiling on both.

**One-line sequencing rationale:** *weave the cloth (integration) → put one memorable flaw in
each bolt (surprise) → then widen the loom (breadth).* Weaving first is what makes the other
two read; doing breadth first just makes a bigger stack.

---

## Appendix — dependency on the Music Theory lens (explicit)

Every S63 lever that touches pitch, harmony, or voice-leading legality — the harmonic-turn
surprise, the registral-break, the walking/pedal bass selection, the pitch↔rhythm character
co-variation (I5) — must be adjudicated by the Music Theory lens for in-key resolution, voice-
leading legality, and clean transposition under section key offsets. **This aesthetic lens owns
*which dimensions to integrate, where surprise may live, and the bar they must clear*; the
Music Theory lens owns *keeping the integrated, surprising result legal*, and owns the breadth
inventory (library sizes + per-image utilization).** A slice is correct only when both hold:
integrated + surprising + distinct to the ear (this doc) **and** legal + coherent to the theory
(theirs). The rhythmic-complementarity and density-response levers (I2/I3) are largely
theory-neutral (they place onsets/rests, not pitches) and are the safest place for the
integration front to start.

---

## S63 gate-close review — Slice 1 (INTEGRATION) against the H1–H8 bar

**Lens:** Composition & Songwriting Aesthetics (trained ear), standing taste voice. This is the
cadence-closing pass: the INTEGRATION slice has landed, and this section judges the *actual*
result against the bar this same doc set above — so the taste gate closes on the real output,
not on the design intent. READ-ONLY; no code was authored here.

**What Slice 1 shipped (as delivered):** the per-piece `RhythmMotto` is now the shared rhythmic
DNA that reaches all three roles. The accompaniment re-phases to the **complement** of the melody
gait (comp answers in the melody's gaps for motto cells 0/1/2; keeps the downbeat to anchor the
syncopated cell 3) and thins inversely to melody activity; the bass derives its pattern
(pedal / walking / pedal_dom) from the motto cell and anchors the downbeat the comp vacates —
the classic bass-anchor / comp-offbeat interlock. Accompaniment animation went **1/6 → 6/6
images**; bass **1/5 → 3 distinct kinds**. Delivered per-image state:

| image | motto cell | accompaniment base figure | bass |
|---|---|---|---|
| Img1 | 1 | arp_waltz (+motto-complement) | pedal |
| Img2 | 0 | arp_waltz (+complement) | walking |
| Img3 | 3 | arp_waltz (+complement) | walking |
| example | 2 | broken_up (+complement) | pedal_dom |
| Lena | 0 | arp_waltz (+complement) | walking |
| magic | 3 | broken_wave (+complement) | walking |

The H3 interlock is **test-proven**: non-cell-3 pieces emit ZERO comp onsets at the melody's
downbeat (they interlock in the gaps); cell-3 pieces anchor the downbeat (the syncopated melody
vacates it, so an on-downbeat comp/bass still *fills the melody's gap* — interlock, inverted).

### Scorecard — AFTER vs the bar

- **H1 — Texture-identity (mute the melody, rate SET 1–5, PASS ≥4) `[ear]`.** **CLEARED, at the
  floor (predicted ~4, up from ~2).** The identity work with the melody muted is now carried by
  the motto-*complement* comp rhythm (distinct per cell) plus the per-piece bass kind, on 6/6
  animated accompaniments versus 1/6 before. The shared arp_waltz *base* caps the headroom but
  does not sink it — see the H1 ruling below. Weak point: same-motto-cell pairs.
- **H2 — Clap-the-arrangement (count distinct claps, PASS ≥4 of 6) `[ear]`.** **CLEARED, at the
  floor (~4 distinct).** The combined surface is now melody-motto ∪ complement-comp ∪
  cell-derived bass, which is genuinely cell-specific — this is the criterion that catches
  "tempo varied but rhythm shared," and Slice 1 moves it from FAIL to pass. The four distinct
  motto cells (0,1,2,3) yield four distinct claps; the two same-cell+same-bass pairs
  (Img2/Lena cell 0 + walking; Img3/magic cell 3 + walking) clap alike, which is why it lands
  at 4 of 6 and not higher.
- **H3 — Interlock (PASS ≥5 of 6 read interlocking) `[ear]``[prop]`.** **CLEARED — strongest
  result, 6/6, test-proven.** This is the buildable proxy for "every part has its own part,"
  and it is the property Slice 1 most decisively wins.
- **H4 — Name-the-surprise (PASS ≥5 of 6, no shared kind+locus) `[ear]`.** **DEFERRED (out of
  slice scope), correctly.** Surprise is the SECOND front (§5); Slice 1 is INTEGRATION only. No
  curated exception was added, so no piece has a nameable surprise yet. This is not a failure of
  the slice — it is the next slice's payload.
- **H5 — Blur / whole-piece distinctness (Blur_after ≥4 AND ≥2 above before) `[ear]`.**
  **BORDERLINE.** The *delta* (≥2 above before) is plausibly cleared — integration is a large
  move from a low, stacked base (~2 → ~3.5–4). The *absolute* ≥4 floor is at risk, because two
  things still cap the whole-piece read: the shared arp_waltz base on 4/6, and the absence of a
  per-piece surprise (H4 deferred). Expected for a first-of-three slices; H5 is designed to
  fully clear only after surprise and breadth land.
- **H6 — Sibling check (no pair overlaps on ≥2 of {texture-feel, rhythmic surface, surprise})
  `[ear]`.** **NOT-CLEARED — this is the honest weak point.** Img2↔Lena overlap on texture-feel
  (both arp_waltz + walking) AND rhythmic surface (both motto cell 0) = 2 of 3 → fails the bar.
  Img3↔magic is borderline (share motto cell 3 + walking bass, but base figures differ:
  arp_waltz vs broken_wave), so ~1–2. See the sibling ruling below.
- **H7 — Coherence floor `[prop]`.** **CLEARED.** The integration is deterministic — the
  complement is computed from the melody gait, the bass from the motto cell; no randomness was
  introduced, and every base figure (arp_waltz / broken_up / broken_wave) and bass kind
  (pedal / walking / pedal_dom) is from the existing curated vocabulary. The I2/I3 levers place
  onsets/rests, not pitches, so they are theory-neutral as predicted.
- **H8 — Per-image stability `[ear]`.** **CLEARED by construction.** The integration levers are
  image-driven (motto cell from edge_activity/complexity; bass from cell), not seed-driven, so
  adjacent seeds render the same DNA / interplay. *Caveat, pre-existing:* harmonic progression
  is still drawn with `thread_rng` (diagnostic #5/#10) — not introduced by this slice, but it
  means adjacent seeds can differ harmonically until that draw is seeded. Out of Slice-1 scope;
  flag only.

**Integration-bar tally:** the three integration-specific properties — H1 (texture-identity),
H2 (arrangement-clap), H3 (interlock) — all move from fail to clear, and H3 clears at 6/6
test-proven. Those are exactly the dominant rank-1 (stacking) and rank-2 (shared rhythmic
surface) causes this slice was scoped to attack. H7/H8 (the balance guards) hold. H4/H5/H6 are
the holistic-bar axes that later slices own.

### Ruling on H1 — will "mute the melody, can you still tell the piece?" land YES?

**YES for the majority (4 of 6); NO for the same-motto-cell pair — net SET ~4, a floor pass.**
The shared arp_waltz base on 4/6 looks fatal to H1 but is less so than it reads, because the ear
identifies an accompaniment by its *rhythmic behaviour* — does it fill, where does it pulse, does
it recede under an active melody — far more than by its arpeggio-vs-broken-chord *color*. Slice 1
put the rhythmic behaviour (the motto-complement, distinct per cell) and the bass kind on
per-piece footing, and those are what carry identity with the melody gone. Where it breaks is the
**same-motto-cell** case: Img2 and Lena share cell 0, so they share the complement rhythm AND the
arp_waltz base AND the walking bass — with the melody muted they are separated only by ensemble
(CounterMelody vs HarmonicFill), a thin axis. So my honest trained-ear prediction: 4/6 are
tellable-apart with the melody muted; the cell-0 pair blurs; the SET rating lands right at the
PASS floor (~4), a genuine but not comfortable clear. The shared base is a *ceiling* on H1, not a
floor-breaker — but it is the ceiling the next slice should raise.

### Ruling on the Img2↔Lena sibling

**Too alike under my own bar — it fails H6 as-is.** Overlap on texture-feel (arp_waltz + walking)
and on rhythmic surface (motto cell 0) = 2 of 3, and the third axis (surprise) is empty for both,
so there is nothing yet to break the tie. The only real differentiator is ensemble role
(Img2 = CounterMelody, Lena = HarmonicFill). **What the operator should listen for:** with the
melody *in*, whether the CounterMelody vs HarmonicFill gives the two pieces distinct upper-voice
motion (a moving second line vs a static harmonic wash) — that difference is real but subtle;
with the melody *muted*, expect them to blur. This pair is the visible seam Slice 1 leaves behind
and the most useful single A/B for judging the next slice: it is cheapest to break by either
(a) giving same-cell pieces a different base figure or bass kind, or (b) the deferred per-piece
surprise, whose unique kind+locus clause breaks any sibling by construction.

### Next-lever recommendation

**Pull the diagnosed texture-threshold re-tune forward as the next lever — it is the thing the
ear flags next, and it is cheap and pinpointed.** Integration alone converts stack → texture (the
dominant fix) but leaves the base-texture *color* shared on 4/6; having gotten past the
rhythmic-behaviour sameness, the ear's next complaint will be "Img1/2/3/Lena are all the same
arpeggiated wash." The gate-liveness scorecard already located the exact cause and the exact
lever: three richer texture profiles (`pad_figured`, `pad_block_comp`, `pad_stride`) carry
thresholds *above* the entire observed calm-corpus feature range, so they are unreachable —
identical selector-starvation to what S62 fixed for pitch and what integration itself just fixed
for rhythm. Opening those three profiles to the calm cluster is a **routing un-gate of existing
curated material**, *not* the open-ended "grow the loom" breadth the §5 sequencing deferred to
last — so recommending it before surprise is consistent with the doctrine's spirit (open routes
to authored material; never synthesize from a hash). It also directly kills the Img2↔Lena sibling
by letting same-cell pieces take different base color. **Recommended order:** (next slice)
texture-threshold re-tune to reach the 3 dead profiles → widens base-figure breadth + breaks the
same-cell siblings; (then) the SURPRISE front (§5 SECOND) for per-piece memorability (H4), which
also hard-breaks any residual sibling. Surprise still follows; the only re-order is pulling the
*targeted, diagnosed* breadth un-gate ahead of it because the scorecard turned it into a known
cheap win.

### Operator A/B protocol for S63 (ready to run against before/after WAVs)

Evaluation set: the six bundled images, **fixed seed**, rendered **BEFORE** (pre-Slice-1) and
**AFTER** (current). Run these three ear checks:

1. **H1 — mute-the-melody.** Mute the melody track on all six; play accompaniment + bass
   back-to-back, BEFORE then AFTER. Rate each SET 1–5 on "each accompaniment has its own
   identity." *Pass:* AFTER ≥ 4. *Expect:* a clear jump from ~2 → ~4, with the cell-0 pair
   (Img2/Lena) as the one that still blurs.
2. **H2 — clap-the-arrangement.** For each piece clap the combined surface (melody + comp + bass
   hits); count distinct claps across the six, BEFORE then AFTER. *Pass:* ≥ 4 of 6 distinct AFTER.
   *Expect:* BEFORE collapses toward the melody-only clap (tempo-varied, rhythm-shared); AFTER
   gives ~4 distinct, one per motto cell.
3. **H3 / H6 — the Img2↔Lena sibling.** Play Img2 and Lena side by side, AFTER, both full and
   melody-muted. Full: listen for whether CounterMelody (Img2) vs HarmonicFill (Lena) gives
   distinct upper-voice motion. Muted: expect them to read as near-siblings — this is the seam
   the next slice must break, and the single most informative A/B for judging it.

**Public-repo note:** role language only; no codenames used in this section.

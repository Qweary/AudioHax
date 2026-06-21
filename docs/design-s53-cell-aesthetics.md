# Design — S53 / fix-direction-2 SLICE 1 (D-CELL): COMPOSITION / SONGWRITING-AESTHETICS pass

**Status:** DESIGN ONLY (aesthetic guidance). No source, mappings.json, or test modified. I write no code.
**Lens:** Composition & Songwriting Aesthetics — the WHOLE-PIECE *does-it-land* question (form, arc, pacing, memorability, homecoming) that no correctness gate renders.
**Companion specs read in full:** `docs/design-s53-cell-seam.md` (the Architect's seam — READ FULLY), `docs/spec-s46-figure-ground-metrics.md` (the figure-ground governor I must not break), `docs/design-s50-affect-cutpoints.md` (the cross-piece spread + per-image tuples this slice must preserve and *extend*).
**Affect companion (`docs/design-s53-cell-affect.md`):** ABSENT at write time — I proceed on the seam spec alone, and flag the two places where the Affect specialist's pass is the load-bearing partner to mine (the secondary-divert key DP-A confirmation and the onset-bias *curve*, both already routed to Affect/Music-Theory in the seam §2.3/I-4).
**Standing role:** one of the two taste/affect review voices wired into the S53 build cadence beside correctness (the Specialist Marshaling Gate). This build is in-class generative/aesthetic; the acceptance bar is the operator's re-listen hearing **"more within-piece rhythmic identity."** That bar is mine to defend.

---

## 0. The one-sentence aesthetic frame

A short piece earns its memorability from **one recognizable gesture that recurs and a homecoming that lands** — and the single most reliable way to give a 30-to-90-second image-piece an *identity you can hum back* is a **consistent rhythmic gait**, a "this is how this picture walks." Slice 1 gives every real image exactly that. My whole verdict turns on one distinction the seam already names but I make load-bearing: **a uniform gait is an asset (it is identity); the risk is not the uniformity, it is the gait fighting the macro-shape it should be reinforcing.** So I bless the grain and spend my budget on the guard-rails that keep the gait *serving* the form rather than flattening it.

---

## 1. AESTHETIC VERDICT — uniform-per-piece is the RIGHT slice-1 grain (CONFIRMED, with one bounded relief)

### 1.1 Verdict: CONFIRM uniform-per-piece. It is identity, not monotony.

**A single sustained rhythmic motto across a whole short piece is the correct aesthetic call for slice 1, and I confirm it without reservation as the grain.** The reasoning is compositional, not merely pragmatic:

- **Identity in short forms IS sameness-with-a-purpose.** A motto (Beethoven-5 "fate," the Dies Irae, a Glass cell, a techno four-on-the-floor) is *defined* by recurring unchanged — its constancy is the thing the ear latches onto and remembers. A 4-to-6-section image-piece is far too short to establish *and then develop* a gait; it has time to establish ONE. Asking slice 1 to vary the gait per section would be asking it to develop a theme it has not yet stated. **State first; develop later** is the correct order, and slice 2 (D-METER) is exactly "later." The seam's instinct here is musically right.

- **The operator's own ask is for a per-piece gait.** The stated acceptance bar — "each image its own gait," "more within-piece rhythmic identity" — is *literally a request for a per-piece constant.* Within-piece *variation* is a different (and later) deliverable; conflating them now would dilute the very signal the operator wants to hear land. A uniform motto is the most direct possible answer to "give this picture its own walk."

- **The macro-shape already supplies the within-piece contrast.** This is the load-bearing point and the reason monotony is NOT the real risk. The piece is *not* rhythmically flat under a uniform motto, because the existing form spine already moves underneath it: per-section **density** varies (the `ctx.section.density` nudge at chord_engine.rs:2092 pushes onset busyness up/down by section), the **phrase contour** swells and tapers, the **cadence ring** relaxes to a single sustained note at every phrase close, and the **articulation curve** breathes with per-bar edge. A uniform *gait* riding a varying *density/contour/cadence* spine is precisely the texture of real music — one consistent groove, dynamically shaded across sections. The motto is the *constant character*; the form is the *variation*. Uniformity of the motto does not produce uniformity of the piece.

**The grain is correct. Do not add per-section gait variation in slice 1.** Doing so would (a) pre-empt slice 2's whole reason to exist, (b) muddy the per-piece identity signal the operator is listening for, and (c) introduce per-section gait choices with no driver yet designed to make them musical (slice 2 builds that driver — beat-strength per bar). Hold the line.

### 1.2 The ONE bounded relief I do recommend — and it is NOT per-section gait variation

There is exactly one place a *sustained* uniform motto risks reading as mechanical rather than characterful, and it is structural, not sectional: **the boundaries — cadences and phrase-ends.** A gait that keeps stamping its onset signature *through* the homecoming will fight the repose. The relief is therefore **not "vary the cell per section"** (rejected above) but **"suppress / relax the motto bias AT structural boundaries"** so the form's existing repose still lands. This is guard-rail GR-2 below; I name it here so the grain verdict is unambiguous: **uniform across sections, relaxed at boundaries.** That is the whole of the relief, and it is a *preservation* of existing behavior, not a new variation axis.

---

## 2. GUARD-RAILS — the encodable "pleasing" constraints the Producer (Music Theory Specialist) MUST honor

These are the constraints that keep "give every piece a strong gait" from breaking the gains it rides on. Each is written as an encodable rule, not a vibe. They are ranked: **GR-1 and GR-2 are HARD (a build that violates either is a defect, recoverable only by belated taste review, not by re-baseline); GR-3 and GR-4 are calibration constraints.**

### GR-1 (HARD) — the motto must NOT invert figure-ground; the melody stays the figure

**The constraint.** The motto biases onset placement *within* the band the band-ladder already chose for a voice (seam I-4/I-5). It MUST route *through* the existing `melody_activity_class` governor (chord_engine.rs:1136), never around it. A background/counter/pad voice's motto-biased onsets must **never** raise that voice's `ActivityClass` to meet or exceed the melody's. **Encodable rule:** after the motto bias is applied, `bg_recession_violations` (spec-s46 F5b — the per-step count of bed-role pairs where `bed_onsets > melody_onsets`) MUST be `<= s46_recession_bound(image)` — i.e. the motto introduces **zero new** recession violations beyond the documented pre-fix residual. If the motto adds even one, it has inverted the hierarchy on that step and the build is defective.

**Why this is the first guard-rail.** The S46 work made the melody the figure by making it the *most active* line, not merely the loudest (the metric-rigidity guard in spec-s46 §0.4: "a build that wins by turning the melody up scores WORSE"). A *rhythmic* motto operates on exactly that axis — onset density — so it is the single intervention most able to silently undo S46. A busy/syncopated motto (cell 2 or cell 3) stamped uniformly onto a *background* voice could give a pad more onsets-per-step than a SUSTAINED melody on a calm image — the precise F1-inversion S46 exists to forbid. **The motto's gait must read in the FIGURE (the melody) first and most; the bed inherits the gait only insofar as it can without out-moving the line.**

**The pleasing-music statement of it:** the gait is the *melody's* walk. The accompaniment grooves *with* it, never *over* it. If a listener's ear is pulled to the bass's rhythm instead of the tune's, the figure-ground has inverted and the piece sounds wrong even if every note is "correct."

### GR-2 (HARD) — the motto bias MUST relax at cadences and phrase-ends; the homecoming must still land

**The constraint.** At every cadence step (`is_cadence`) the motto's onset bias MUST be **fully suppressed** (the bias term goes to zero); at the pre-cadence approach and phrase-end taper it MUST be **attenuated**, not full-strength. **Encodable rule:** the motto onset-bias multiplier is `0.0` when `is_cadence`, and scaled down (toward but not to zero — a partial taper) when `pre_cadence` (the `step.position_in_phrase + 2 >= step.phrase_len` window at chord_engine.rs:2104) is true. Concretely, the motto bias must compose *before* the existing cadence early-return at chord_engine.rs:2234 and *must not perturb the single sustained ritardando ring* — the cadence still emits one legato, ritardando-lengthened note, exactly as today.

**Why this is HARD.** This is the structural relief from §1.2 and it is what makes the homecoming readable. The cadence ring (chord_engine.rs:2234–2261) is the piece's *repose* — "the arrival is a point of repose, not an active figure" (the engine's own comment at :2233). The phrase-end ritardando (`RITARDANDO_FACTOR` at :2210) is the music *relaxing into* that arrival. A gait that keeps subdividing or syncopating *through* the cadence would do to the homecoming what a drummer who refuses to ritard does to a final chord: it robs the return of its sense of *home*. In a short form the homecoming is most of the emotional payoff; a motto that flattens it trades a little surface energy for the whole point of the form. **The gait is what the piece DOES; the cadence is where it RESTS. The motto must let it rest.**

**The pleasing-music statement of it:** every piece needs to breathe at its phrase-ends and arrive at its close. The gait drives the body of the phrase; it gets out of the way for the cadence. A consistent gait that *also* knows when to relax reads as *musical*; a gait that never relaxes reads as a *loop*.

### GR-3 (calibration) — the per-piece motto must ADD to the S50 cross-piece spread, never muddy it

**The constraint.** The motto is selected from `edge_activity` (spread) + `affect_arousal` (seam §2.3) — the *same* axes that already drove the S50 (band, cell, character) per-image tuples. The risk is **redundancy collapse**: if the motto's cell merely re-states the band the band-ladder already chose, it adds no new identity and may even *narrow* perceived distinctness by doubling down on one axis. **Encodable rule:** across the six probe images, the *realized* per-piece motto cells must not collapse the existing 6/6 distinct (band, cell, character) tuples (design-s50 §4) — the post-slice tuple set, now including the *honest* per-piece `Section.motto.cell_index` (replacing the always-`NO_THEME_CELL` observable, per seam §5.2-B), must stay at **≥ 4 distinct cells occupied across the six** AND must not reduce the count of distinct *rhythmic signatures* below the S50 floor. A probe-pair whose motto cells *collapse* (two images that S50 separated now landing the same band AND the same motto cell AND the same character) is a driver-tuning failure to fix, **not** a diff to accept.

**The specific muddying to watch — the cell-3 over-application, carried forward from S50 §7 tension #2.** design-s50 already flagged that the complexity-gated cell-3 (the syncopated/profiled gait) fired on THREE of six images (Img3, example, magic) and warned this could read as "the same trick three times." The slice-1 motto keys its secondary divert on `affect_arousal` instead of raw complexity — which is the *right* fix in principle — but the Producer must verify the realized result does not re-cluster three or more probes onto the *same* motto cell. **Encodable check:** if ≥ 3 of the six probes land the same `motto.cell_index`, the secondary-divert cut (`PIECE_AROUSAL_PROFILED`) is over-applying and must be raised until the syncopated gait is *selective* (≤ 2 probes), exactly as design-s50 §5 prescribed dialing `CELL_COMPLEXITY_PROFILED` toward 0.23. The gait that defines a piece must not be the gait that defines *half* the catalogue.

**The pleasing-music statement of it:** six pictures, six walks. If three of them walk the same way, the slice has manufactured a new sameness in the act of curing the old one.

### GR-4 (calibration) — the gait must be AUDIBLE: minimum perceptible contrast (the payoff)

**The constraint.** See §3 — the audible-contrast target. Stated here as a guard-rail: a motto whose onset bias is too subtle to hear is not a defect of correctness (it passes every gate) but a defect of *deliverable* — it fails the operator's re-listen acceptance bar. **Encodable rule:** the motto onset bias must shift the realized per-step onset *count or placement* by a perceptible margin on the body (non-cadence, non-phrase-start) steps — see §3 for the floor. A motto that the spread/figure-ground guards have quietly squeezed down to inaudibility has been over-constrained and must be re-opened (within GR-1/GR-2) until it reads.

---

## 3. THE PAYOFF TEST — will a listener hear each piece's OWN gait? + the audible-contrast target

### 3.1 The test, image by image (grounded in the S50 tuples)

The honest question: across the six probes, after un-gating, will the ear hear **six distinguishable rhythmic characters**, or will the gaits be too subtle/too similar to register? Walking the S50 per-image landing (design-s50 §4) and reading each cell's *authored gait* from the vocabulary (chord_engine.rs:3242–3314):

| image | S50 band | motto driver (edge→cell, arousal→divert) | the gait the listener should hear | distinct? |
|---|---|---|---|---|
| **magicstudio** | SUSTAINED | low edge, mid-high arousal → broad/profiled | slow, spacious, sostenuto — *held, ceremonial* | yes — the slowest walk |
| **AudioHaxImg1** | SUSTAINED | lowest edge, low arousal → **broad (cell 1)** | augmented, sighing — *the calmest, most open gait* | yes vs magic if cell differs (broad vs profiled) |
| **Lena** | DOTTED | mid edge, mid arousal → **anchor (cell 0)** | the S39 endpoint-framed gait — *even, balanced* | yes — the "neutral" walk |
| **AudioHaxImg3** | DOTTED | mid edge, higher arousal → **profiled (cell 3)** | dotted/lilting, long-short — *a sprung, characterful walk* | yes vs Lena (the dotted lilt vs the even anchor) |
| **AudioHaxImg2** | SYNC | higher edge → **busy (cell 2)** | even running quavers — *the brisk, driving walk* | yes — the most "moving" of the mid group |
| **example** | ARPEGGIO | highest edge, high arousal → **busy/profiled** | the fastest, most subdivided — *the energetic outlier* | yes — clearly the busiest |

**My verdict on the payoff: YES, this slice will produce audible per-piece identity — conditionally.** The *spread* is there: the six images genuinely span the gait vocabulary from sostenuto (magic/Img1) through even-anchor (Lena) and lilting-dotted (Img3) to running (Img2/example). The condition is GR-4 + GR-3: the bias must be set strong enough to *clear the floor of perceptibility* (§3.2) and the cell-3 divert must stay selective (GR-3) so Img3/example/magic don't collapse onto one syncopated gait. If those two hold, a trained ear on a re-listen will hear each picture walk differently — which is exactly the operator's bar.

**The honest risk I will not paper over:** the two SUSTAINED images (magic, Img1) and the two DOTTED images (Lena, Img3) are the pairs most at risk of reading as "similar." Their separation rests entirely on the *cell* differing within a shared band (broad-vs-profiled for the SUSTAINED pair; anchor-vs-profiled for the DOTTED pair). That is precisely the axis this slice introduces — so this slice is *what makes those pairs separable* — but it means the motto bias for cells 0/1/3 must be **distinct enough from each other**, not just distinct from "no motto." If the broad gait and the profiled gait, under a *uniform* application across a sparse SUSTAINED piece, sound nearly the same to the ear, the SUSTAINED pair collapses. **This is the single ear-test the Producer must run first** (see §3.3).

### 3.2 The audible-contrast target (the minimum the Producer should aim for)

The bias must be **perceptible on the body steps but invisible on the boundaries** (GR-2). The minimum perceptible target, expressed in the engine's own onset-count terms (the figure-ground metric's unit, spec-s46 F1 — "onsets per step"):

- **Floor (must clear, or it is inaudible):** the motto must change the realized **melody** onset profile on the body steps by at least the perceptual equivalent of **one onset-class step** — i.e. a busy/cell-2 motto should push the melody at least one band busier in realized onsets than a broad/cell-1 motto would, on the *same* image's body steps. A bias smaller than "one band's worth of onset difference between the broadest and busiest cell" will not register against the already-varying density/articulation spine. This mirrors the S46 F1 SUBJECT-image margin (+0.3 onsets/step) as the order of magnitude the ear demonstrably resolves.
- **Ceiling (must not exceed, or it breaks GR-1/GR-3):** the bias must NOT push any *bed* voice's onsets up by enough to approach the melody's (GR-1), and must NOT push a mid-cluster image into the busiest gait when S50 placed it mid (the over-drive guard, design-s50 §6 — the same "computer-like fragmentation" the operator already disliked). The bias is a *re-distribution within the band*, not a band promotion.
- **Where to spend the contrast:** on the **figure (melody) onset placement**, on the **body steps**, between the cells *themselves* — broad vs anchor vs busy vs profiled must each read as a recognizably different walk. The contrast that matters for "each picture its own gait" is **cell-vs-cell**, not merely "motto-vs-no-motto." The Producer should ear-test the four cells of a *single* archetype back-to-back on one image and confirm broad/anchor/busy/profiled are four hearable gaits before trusting that six images will separate.

### 3.3 The first ear-test the Producer must run (the marshaling gate's standing ask)

Before the cuts are frozen, render and *listen to* the two at-risk same-band pairs:
1. **The SUSTAINED pair (magic vs Img1):** does the profiled gait (magic) read as *distinct* from the broad gait (Img1) when both sit in a sparse, slow, sustained texture? If they sound the same, the motto is inaudible *in the sparse register* — the place it is hardest to hear — and either the bias must open (GR-4) or the SUSTAINED-band images need the divert to *not* both land non-anchor.
2. **The DOTTED pair (Lena vs Img3):** does the even-anchor (Lena, cell 0) read as distinct from the dotted-lilt (Img3, cell 3)? This pair is the *best case* for audible separation (anchor vs profiled is the largest gait contrast within a band) — if even *this* pair doesn't separate, the bias is too weak globally.

This is the ear-test, standing beside correctness in the build cadence, that this whole pass exists to demand. It is not an optional end-of-slice check; it is the gate.

---

## 4. PRESERVATION TENSIONS — where "strong gait" pulls against the preserved gains

The three gains to preserve are **S46 figure-ground (6-VARIED), S50 cross-piece spread, S52 honesty.** I flag the genuine tensions, not boilerplate:

### 4.1 TENSION (S46 figure-ground) — strongest gait vs flattest hierarchy. RESOLVED BY GR-1.
The more *strongly* the motto stamps its onset signature on every voice (the instinct of "give it a strong gait"), the more it risks the bed inheriting enough of the gait to out-move a holding melody. **These pull against each other directly.** Resolution: the gait lives **in the melody first** and the bed inherits it only under the F5b governor (GR-1). The gait can be as strong as you like *on the figure*; it is *clamped* on the ground. There is no way to honor "strong per-piece gait" except by making it the *melody's* gait — which is also, not coincidentally, the most musical reading (the tune carries the groove). **No real conflict once the gait is correctly located in the figure; a real conflict if it is applied flat across all voices.**

### 4.2 TENSION (S50 cross-piece spread) — within-piece identity vs between-piece distinctness. RESOLVED BY GR-3.
The motto reads from the *same* axes (spread edge + arousal) that produced the S50 between-piece spread. Done naively, the motto is *redundant* with the band the spread already chose — it adds no new information and risks the cell-3 re-clustering (design-s50 §7 #2). Done well, the motto adds a *third, independently-realized* dimension (the realized onset *placement* gait, distinct from the band the ladder picks and the character the tempo picks). **The tension is real:** same input, must produce *additive* not *redundant* output. Resolution (GR-3): keep the secondary divert selective (≤ 2 probes on cell 3) so the motto *decorrelates* the same-band pairs (it is *what* separates magic/Img1 and Lena/Img3) rather than re-stating their shared band. **The slice succeeds precisely when the motto is the thing that pulls the same-band pairs apart — that is additive; it fails if it just re-says the band.**

### 4.3 TENSION (S52 honesty) — NO tension; the slice STRENGTHENS honesty. (flagged for completeness.)
S52 honesty = "the tree reflects what the engine actually reads." Today the rhythm-variety observable reads `themes[0]` which is *always empty* on real photos (the dead axis) — an honesty *gap* (a cell that is "selected" but never reaches the realizer). After the slice, the observable is `Section.motto.cell_index`, which the realizer *actually reads* (seam §5.2-C). **The slice removes a dishonest dead-axis observable and replaces it with a live one — honesty improves.** The only discipline (already in the seam): re-point the test machinery at `Section.motto`, with the §5.2-B justification hunk, and do NOT re-baseline the Class-A engine-kernel goldens (the neutral-motto freeze hinge holds them byte-stable). No aesthetic tension; I flag it only so "preserve S52" is not mistaken for "don't change the test" — the test *should* change, toward more honesty.

### 4.4 The residual aesthetic worry I am NOT resolving here (correctly out of scope) — the lament/empty edge.
design-s50 §7 #4 flagged that Img1 → lament (slowest window) + SUSTAINED band risks reading as *empty* rather than *calm*. A uniform *broad* motto on that already-sparse piece could deepen the emptiness (a broad gait subtracts onsets). **I flag this as a watch-item, not a slice-1 fix:** within-piece *density* is fix-direction-2's slice-2 (D-METER) territory, and the motto's job is identity not density. But the Producer should confirm at the ear-test that the broad motto on Img1 does not tip "calm" into "barren." If it does, GR-2's boundary-relaxation is *not* the lever (that's about cadences); the lever is slice 2, and the right move is to log it forward, not to over-drive the slice-1 motto to compensate.

---

## 5. SUMMARY FOR THE PRODUCER (the encodable checklist)

1. **Grain:** uniform-per-piece motto, ALL sections same cell. Do NOT add per-section variation (that's slice 2). ✔ confirmed correct.
2. **GR-1 (HARD):** motto bias routes through the `melody_activity_class` governor; `bg_recession_violations` gains ZERO new violations (spec-s46 F5b). The gait is the *melody's*; the bed grooves with it, never over it.
3. **GR-2 (HARD):** motto bias = 0 at `is_cadence`; attenuated at `pre_cadence`/phrase-end; the cadence ring (chord_engine.rs:2234) emits its single sustained ritardando note UNPERTURBED. The gait drives the phrase body and gets out of the way for the homecoming.
4. **GR-3 (calibration):** keep the cell-3/syncopated divert SELECTIVE — ≤ 2 of the six probes on any one motto cell; preserve the S50 6/6 tuple distinctness (≥ 4 cells occupied). The motto must PULL same-band pairs apart, not re-state their band.
5. **GR-4 / payoff (calibration):** the bias must clear ~one-onset-class of audible contrast on the melody body steps (the S46 F1 +0.3-onsets/step order of magnitude), cell-vs-cell, without band-promoting a mid image (design-s50 §6 over-drive guard).
6. **First ear-test (the marshaling gate's standing ask):** listen to the SUSTAINED pair (magic vs Img1) and the DOTTED pair (Lena vs Img3) BEFORE freezing cuts — confirm broad/anchor/profiled are hearable as different walks in the sparse register.
7. **Affect-partner handoffs (companion doc absent at write time):** the secondary-divert KEY + cut value (`PIECE_AROUSAL_PROFILED`) is the Affect specialist's DP-A call (seam §2.3); the onset-bias CURVE (cell weights → onset offsets) is the Music Theory Specialist's call (seam I-4). My pass fixes the *constraints* those choices must satisfy, not the numbers.

---

*End of S53 D-CELL aesthetics pass. Design-only: no src/asset/test modified. Companions: `design-s53-cell-seam.md`, `spec-s46-figure-ground-metrics.md`, `design-s50-affect-cutpoints.md`. The acceptance bar is the operator's re-listen hearing more within-piece rhythmic identity; GR-1/GR-2 are the HARD guard-rails that keep the gait serving the form, GR-3/GR-4 the calibration that keeps it audible and additive.*

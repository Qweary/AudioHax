# Cross-Piece Sameness vs. Distinctiveness: A Cited Principle Brief

**Purpose.** This document assembles the music-theory principles that bear specifically on *why a body of generated pieces can all sound like one archetype* — the listener complaint of "sameness," and in particular "the same common rhythmic feel across every piece." It is not a general theory survey. Every principle here is selected because it distinguishes a piece that **develops and varies** its material from one that **restates and loops** it, and because it discriminates between pieces at the level a listener actually perceives.

It is organized on five axes: (1) melodic development / motivic transformation, (2) harmonic vocabulary beyond diatonic, (3) form and harmonic-rhythm variety, (4) groove / feel / rhythm, and (5) arrangement / orchestration arc. For each axis the brief states the core principles precisely, then gives a **diagnostic test** — the concrete question an auditor asks of a generative system to decide whether the principle is **IMPLEMENTED**, **SHALLOW-FAKED** (present by name but applied so uniformly or thinly it does not register perceptually), or **ABSENT**. A closing synthesis ranks the axes by their likely contribution to a "sameness" verdict and evaluates one architectural hypothesis.

A recurring theme runs through all five axes and should be held in mind from the start: **the difference that produces perceived distinctiveness is a difference of *kind*, not a difference of *value on a fixed parameter*.** Re-rolling a parameter inside a single fixed structure changes the notes; it does not change what the ear recognizes as the structure. This distinction is the analytical spine of the whole document.

---

## Axis 1 — Melodic Development / Motivic Transformation

### Core principles

The foundational distinction, due to Schoenberg, is between **restatement** and **developing variation**. A motive is the smallest structural unit that recurs and is felt as characteristic; a piece acquires the sense of growth and forward motion only when successive statements of the motive are *varied* rather than merely *repeated*. Schoenberg names the homophonic style itself "the style of developing variation," defining it as the condition in which "in the succession of motive-forms produced through variation of the basic motive, there is something which can be compared to development, to growth" (Schoenberg, *Fundamentals of Musical Composition*, 1967). Exact repetition, by contrast, "is the simplest" relationship and, used alone, produces monotony; Schoenberg's entire pedagogy of the motive is a catalogue of ways to keep the motive recognizable while changing it.

The standard operations of motivic transformation, as taught in common-practice theory and in the Berklee melodic-writing lineage (Perricone, *Melody in Songwriting*, 2000), are:

- **Sequence** — restating a motive at a different pitch level. **Real** sequence transposes intervals exactly (often tonicizing); **tonal** (diatonic) sequence preserves scale degrees, so interval qualities change. Sequence is the workhorse of continuation.
- **Inversion** — reversing melodic direction (ascending intervals become descending). May be strict (interval-exact) or tonal (diatonic).
- **Retrograde** — stating the pitch (and/or rhythm) sequence backwards.
- **Augmentation / diminution** — proportionally lengthening or shortening the rhythmic values of the motive while keeping its contour. This is a *rhythmic* transformation of a melodic idea and is one of the strongest agents of variety because it changes surface pace without changing identity.
- **Fragmentation** — isolating a sub-portion of the motive (e.g., its head or tail) and working with it alone. Schoenberg treats fragmentation as the primary engine of the development section.
- **Extension / liquidation** — its complement. **Liquidation** is Schoenberg's term for the gradual stripping-away of a motive's characteristic features until only neutral, non-committal residue remains (his classic illustration reduces a motive-complex *a–b–c* to *b–c*, then to *c* alone), which is how a passage dissolves toward a cadence rather than perpetuating itself. **Extension** stretches the idea by internal repetition or spinning-out.
- **Interval expansion / contraction** — widening or narrowing the characteristic intervals of the motive (a rising third becomes a rising sixth), preserving rhythm and contour direction while changing affect and register.
- **Motivic saturation / *Grundgestalt* thinking** — the ideal in which nearly all melodic material of a piece is demonstrably derived from one basic shape, so the piece feels *unified* (all-from-one) rather than *uniform* (all-the-same). Unity and uniformity are not the same thing: unity is derivation with transformation; uniformity is restatement without it.

The diagnostic pivot for this axis: **a piece that DEVELOPS material presents the same identity in transformed guises across its span; a piece that LOOPS material presents the same identity in the same guise.** Both are "coherent." Only the first is *interesting over time*, and — critically for cross-piece sameness — only the first has a large combinatorial space of realizations, so two pieces built by development diverge, whereas two pieces built by looping converge on their shared template.

### Diagnostic test

Ask of the generative code:

- **IMPLEMENTED:** Is there a motive object that is *transformed* by named operations (sequence, inversion, augmentation, fragmentation, interval-expansion) as the piece proceeds, such that later phrases are provably derived-but-different from earlier ones? Does the transformation trajectory itself vary run-to-run (piece A liquidates and fragments; piece B augments and expands intervals)?
- **SHALLOW-FAKED:** Is a "motif" generated and then merely *repeated or transposed*? Verbatim repetition and plain transposition are the two operations most often mistaken for development. If the only transformation applied is transposition (and always the same transposition scheme), the system has motivic *restatement* dressed as development. A tell: the transformation set exists in the code but a single member of it is called every time, or the transformations are applied in a fixed order for every piece.
- **ABSENT:** Are phrases generated independently (e.g., each phrase freshly sampled) with no shared derived material, or is one phrase generated and tiled? Both extremes — no motive, or one looped motive — read to the ear as "nothing is going anywhere." Note that *absence* of development and *shallow* development sound different from within a piece but can sound equally same-y *across* pieces, because both collapse the realization space.

### Contribution to a "sameness" verdict

Motivic monotony is the classic cause of *within-piece* tedium. Its role in *cross-piece* sameness is real but usually secondary to rhythm: if every piece runs the same handful of motivic operations in the same order, the melodic surfaces rhyme. But listeners rarely name melodic-development failure as "same rhythm" — they name it as "it never goes anywhere" or "it keeps repeating." When the complaint is specifically *rhythmic feel*, look to Axis 4 first, and treat weak development as a compounding, not primary, factor.

---

## Axis 2 — Harmonic Vocabulary Beyond Diatonic I–IV–V–vi

### Core principles

A harmonic language restricted to the common diatonic triads — especially the pop-ubiquitous I–V–vi–IV set — will sound like itself in every piece, because the *vocabulary* is the constant and only the *ordering* varies, and ordering a tiny set yields perceptually near-identical results. Harmonic variety across pieces requires expanding the vocabulary, not just permuting a fixed one. The Berklee chord-scale tradition (Nettles & Graf, *The Chord Scale Theory & Jazz Harmony*, 1997) and common-practice chromatic-harmony pedagogy supply the standard expansions:

- **Secondary dominants (V/x) and secondary function** — applying a dominant (or ii–V) to a diatonic target other than the tonic (V/V, V/vi, etc.). This introduces chromatic tones (the tonicizing leading tones) and directional pull toward internal targets. It is the single cheapest, highest-yield expansion beyond pure diatonicism.
- **Modal interchange / borrowed chords** — importing chords from the parallel mode (e.g., in major: bVII, iv, bVI, ii°, the minor-mode subdominants). Modal interchange changes the *color* of a progression without leaving the key and is a primary source of the difference between a "plain" and a "rich" harmonization of the same melody.
- **Tension extensions (9, 11, 13) and altered dominants** — adding upper-structure tensions and chromatic alterations (b9, #9, #11, b13). These change the chord's *quality and affect* even when its root-motion function is unchanged; they are how two pieces with the same roman-numeral skeleton can sound harmonically distinct.
- **Chromatic approach and voice-leading chords** — passing diminished sevenths, chromatic approach chords, line-cliché motion. These create harmonic motion *between* the structural chords, enriching harmonic surface independent of the underlying functional plan.
- **Modulation and tonicization** — moving the tonal center (or briefly implying a new one). A piece that stays rigidly in one key with one cadence target has less harmonic range than one that tonicizes or modulates; across a body of pieces, a fixed key-plan is a strong sameness contributor.
- **Reharmonization** — the practice of setting the same melody with a different chord sequence. Its existence proves the point of this axis: identical melodic material can be made to sound substantially different purely by changing the harmonic vocabulary underneath it, and vice-versa.

The diagnostic pivot: **the minimum expansion beyond a small diatonic set that a listener reliably perceives as harmonic *variety between pieces* is a change of *chord-quality color and function*, not merely a change of *chord order*.** Empirically and pedagogically, the highest-leverage single additions are (a) at least one secondary-dominant/tonicization mechanism and (b) modal-interchange color; these two alone move a system from "one harmonic accent" to a recognizably plural harmonic palette. Adding tensions deepens the effect but does not, by itself, escape a fixed functional skeleton.

### Diagnostic test

- **IMPLEMENTED:** Does the harmony generator draw from a vocabulary that includes secondary function, borrowed chords, and extensions/alterations, *and* does the vocabulary drawn upon vary piece-to-piece (piece A leans on modal interchange; piece B tonicizes vi)? Are key-plans and cadence targets variable?
- **SHALLOW-FAKED:** Are extensions or borrowed chords *present in the chord tables* but selected by a rule that fires so rarely, or so uniformly, that essentially the same 3–4 chords appear every time? A common shallow pattern: a large chord vocabulary exists, but a fixed progression template (e.g., always I–vi–IV–V, or always the same functional map) selects from it, so the *added* vocabulary never changes the perceived progression. Another tell: extensions are added decoratively to every chord uniformly (everything is a 9th chord), which is itself a new kind of sameness — a uniform color is still a constant.
- **ABSENT:** Is the progression drawn from a small fixed diatonic set with no chromatic function and no key change? If so, harmony is a near-constant across the catalogue.

### Contribution to a "sameness" verdict

Harmonic poverty is a strong *cross-piece* sameness driver and a comparatively weak *within-piece* one — a single good diatonic loop can sustain one piece but makes every piece sound related. However, harmony is not usually what a listener means by "the same *rhythm*." Fixed harmony contributes to a general "these all sound alike" impression; it is unlikely to be the sole cause of a *rhythmic-feel* complaint. Rank it high for global sameness, lower for the specific rhythm complaint.

---

## Axis 3 — Form and Harmonic-Rhythm Variety

### Core principles

Even with varied notes and chords, a piece will sound stamped-from-a-mold if its *phrase structure*, *form*, and *harmonic rhythm* are fixed. This axis governs the temporal skeleton on which the other axes hang.

- **Phrase-length variety.** Common-practice and songwriting pedagogy treat the four-bar phrase and eight-bar period as *defaults to be varied*, not laws. Phrase **expansion** (extension, prefix/suffix, deceptive prolongation) and **contraction**, and asymmetrical phrase lengths (3-, 5-, 7-bar phrases), are the means by which two pieces with the same materials acquire different *breathing*. A system that emits only symmetrical 4+4 phrases imposes one gait on everything.
- **Formal variety.** Beyond a single default form (e.g., always AABA, or always verse–chorus, or always a through-composed sweep of fixed length), form itself is a variable: binary, ternary, rondo, strophic, arch, through-composed, and their proportions. If every generated piece instantiates one formal template, the large-scale shape is a constant the listener will eventually feel even if local content differs.
- **Harmonic rhythm** — the *rate at which chords change* (Wikipedia/pedagogical consensus: "the rate at which chords change," a.k.a. harmonic tempo). Two principles matter for distinctiveness:
  1. **Rate as an independent variable.** One chord per measure, two per measure, one per phrase, and one per beat are *different musical surfaces* even over the same progression. A fixed harmonic-rhythm rate is one of the most insidious sameness sources because it is rarely noticed consciously yet strongly shapes perceived pace.
  2. **Harmonic-rhythm variation within a piece — especially acceleration toward cadence.** Common practice routinely *accelerates* harmonic rhythm approaching a cadence (chords change faster as the phrase closes) and places the cadence chord on a metrically strong beat. A system that holds harmonic rhythm perfectly constant loses this directional cue, so its phrases do not *arrive* — they just stop and restart, which reads as loopiness.

The diagnostic pivot: **a fixed phrase-and-harmonic-rhythm template makes every piece sound the same even when the notes differ, because the template is the part the ear tracks as "the shape of the music."** Melody and harmony are the *content*; phrase length, form, and harmonic-rhythm rate are the *container*. If the container is identical across the catalogue, the catalogue has one shape with interchangeable filling — which is precisely the perceptual signature of "these are all the same piece."

### Diagnostic test

- **IMPLEMENTED:** Do phrase lengths vary (including asymmetrical phrases), does the form vary across pieces, and does harmonic-rhythm rate both differ *between* pieces and change *within* a piece (e.g., accelerate toward cadences)?
- **SHALLOW-FAKED:** Is there a "form" or "phrase" abstraction that is always instantiated with the same length and the same subdivision — e.g., every piece is N bars of 4-bar phrases with one chord per bar? Parameterizing *tempo* or *key* while holding the phrase/harmonic-rhythm grid fixed is the archetypal shallow fake for this axis: the pieces are faster/slower and higher/lower, but the grid — hence the felt shape and pace — is invariant. This is the specific mechanism by which "varying tempo and cadence location does not fix sameness."
- **ABSENT:** Is there no phrase model at all (material generated as an undifferentiated stream) — which paradoxically also produces sameness, because without phrase articulation every piece has the same featureless flow?

### Contribution to a "sameness" verdict

This axis is the **prime suspect co-equal with Axis 4** for cross-piece sameness, and it is the axis that most directly explains *why re-rolling parameters fails*. Tempo, key, cadence placement, and note choice are all *content inside the container*. If the container (phrase grid + form + harmonic-rhythm rate) is a single fixed template, no amount of content variation escapes the felt sameness — the listener is tracking the container. Note the tight coupling to Axis 4: a fixed harmonic-rhythm grid and a fixed rhythmic-feel grid are two faces of one problem — a single stamped temporal architecture.

---

## Axis 4 — Groove / Feel / Rhythm  *(priority axis)*

### Core principles

This is the axis most likely responsible for a complaint phrased as "the same common rhythm across every piece," and it is where the content/container distinction bites hardest. The critical, load-bearing concept is:

> **Changing the TEMPO changes how fast the same feel goes. Changing the FEEL/subdivision changes what the rhythmic surface *is*.** These are different operations, and only the second alters what a listener hears as "the rhythm."

A "groove" or "feel" is a temporal gestalt: the perception of recurring pulses and their **subdivision**, arising from aligned concurrent rhythmic patterns (music-perception literature on groove; e.g., Madison; Witek et al., 2014). Its distinctiveness between pieces is carried by the following, none of which is tempo:

- **Subdivision and the subdivision grid.** Whether the beat is felt in **duple** subdivision (eighths/sixteenths) or **triple** subdivision (triplets/compound meter) is the single largest determinant of rhythmic-surface identity. A straight-eighths feel and a triplet/shuffle feel over the *identical* tempo and *identical* harmony are heard as *different rhythms*. A system that fixes the subdivision grid has fixed "the rhythm" no matter what else it varies.
- **Straight vs. swung feel (swing ratio).** Swing is the systematic lengthening of on-beat vs. off-beat subdivisions (a long–short ratio, canonically near but not exactly 2:1, and tempo-dependent; see Friberg & Sundström; the *Scientific Reports* microtiming-and-swing work, 2019). Straight and swung are categorically different surfaces. Swing ratio and its presence/absence are feel variables, not tempo variables.
- **Syncopation.** Placing accents off the metric grid (on weak beats or offbeats). Syncopation is one of the empirically strongest correlates of groove and of the *desire to move*. Witek et al. (2014, *PLoS ONE*) demonstrate an **inverted-U relationship**: *medium* degrees of syncopation elicit the most pleasure and movement, while too little (metronomic, all on-beat) and too much (disorienting) both reduce it. Two consequences for distinctiveness: (a) a system pinned at near-zero syncopation will sound rhythmically flat *and* uniform across pieces; (b) syncopation *pattern* (where the offbeats fall), not just syncopation *amount*, is a per-piece identity carrier.
- **Rhythmic density and event rate.** How many onsets per beat/measure. Density correlates with groove and perceived energy independently of tempo; a fixed density is a fixed surface even under varied tempo. (Note: several microtiming studies find groove correlates more with event density, beat salience, and syncopation than with microtiming deviation magnitude — density and syncopation are the levers, not micro-timing jitter.)
- **Metric variety.** The meter/time signature and its internal accent pattern (4/4 vs. 3/4 vs. 6/8 vs. 5/4; backbeat placement; the "common time, backbeat on 2 and 4" default). A catalogue locked to one meter with one accent template has one metric identity.
- **Groove/pattern vocabulary.** The characteristic recurring onset pattern (the "beat" — its kick/snare/accent skeleton, or in non-drum textures, the accompaniment rhythmic ostinato). This is the most concrete carrier of "which groove is this." A single pattern template — however it is otherwise ornamented — is heard as *one groove* across every piece.

The diagnostic pivot, stated as sharply as the literature allows: **varying tempo, phrase placement, or cadence location does NOT fix a "same rhythm" complaint, because none of them changes the subdivision grid, the swing/straight feel, the syncopation pattern, the density, the meter, or the groove skeleton — the six things that actually constitute the rhythmic *surface*.** Tempo scales the surface in time; phrase/cadence placement relocates events *within the same surface*. The listener hearing "same rhythm" is reporting an invariant *surface*, and the only fix is to vary the surface-defining variables themselves — most powerfully the **subdivision grid** and the **groove/pattern skeleton**, secondarily swing feel, syncopation pattern, density, and meter.

A corollary the audit must respect adversarially: **no single one of these levers is a guaranteed fix.** Adding syncopation to a fixed straight-sixteenths grid helps but still leaves every piece in straight sixteenths; changing meter while keeping one accent template can still sound same-y. The surface reads as *different* when *multiple* surface-defining variables genuinely differ *and cohere into a recognizably different groove* — i.e., when the system possesses a **vocabulary of distinct feels**, not one feel with adjustable parameters.

### Diagnostic test

- **IMPLEMENTED:** Does the system select among genuinely distinct **subdivision grids** (straight duple vs. triple/compound), among **straight vs. swung** feels with a variable swing ratio, among **groove/pattern skeletons**, and among **meters** — with **syncopation pattern** and **density** as further independent, per-piece variables? Crucially: are these *feel* selections independent of *tempo*, so that two pieces at the same tempo can have audibly different rhythmic surfaces?
- **SHALLOW-FAKED (the most important case to detect):** Is there exactly **one** subdivision grid / one groove skeleton / one meter, with only **tempo, note placement, phrase length, or cadence location** varying on top of it? This is the precise fingerprint of the "same common rhythm" complaint. Symptoms in code: a single hardcoded subdivision (e.g., everything quantized to straight sixteenths); a single accompaniment-rhythm template applied to all pieces; "rhythmic variety" implemented as re-rolling *onset positions within a fixed grid* rather than *changing the grid*; tempo exposed as the primary rhythmic control. Also shallow: syncopation present but pinned at a single low amount for every piece (misses the inverted-U's middle), or a "swing" flag that is always on or always off.
- **ABSENT:** Is there no groove/feel model — rhythm emerges only as a by-product of note durations with no pattern skeleton, subdivision policy, or metric accent structure? Absence tends to produce a different but adjacent complaint ("rhythmically shapeless / mechanical"), and across pieces still converges on a default surface.

### Contribution to a "sameness" verdict

**This axis is the leading suspect for a complaint specifically about "the same common rhythm-feel."** The listener's own words diagnose the axis: rhythmic feel is what they named. The mechanism is the content/container distinction at its sharpest — the rhythmic surface is a *container* variable, and the parameters most systems expose first (tempo, note choice, cadence placement) are all *content* variables that ride *inside* the container. That is exactly why those knobs "don't fix it." The high-leverage fixes are categorical additions to the *feel vocabulary* (subdivision grid, groove skeleton, swing, meter), not finer resolution on existing knobs.

---

## Axis 5 — Arrangement / Orchestration Arc

### Core principles

Two pieces with different notes, chords, forms, and grooves can still sound same-y if their **texture** is a constant: same instrumentation, same voice roles, same register distribution, same density, holding still from beginning to end. Arrangement/orchestration pedagogy (Pease & Pullig, *Modern Jazz Voicings*; Pease, *Jazz Composition: Theory and Practice*, Berklee Press; general orchestration principles) treats texture as a primary expressive and differentiating dimension.

- **Voice roles.** Arranging distinguishes functional roles — melody (lead), bass, harmonic pad/comping, counter-line, and rhythmic/figuration layers. Which roles are present, and how they are assigned to instruments and registers, is a strong identity carrier. A fixed role assignment (always melody-over-block-chords-over-bass) is a textural constant.
- **Textural contrast between sections.** A central arranging technique is *changing the texture* to articulate form — thinning to solo/duo, thickening to tutti, shifting from homophonic block-chords to contrapuntal/linear writing, moving material between registers. Contrast *between* sections is what makes a form feel shaped rather than uniform.
- **Instrumentation / orchestration variety.** The choice and combination of timbres. Even holding composition constant, re-orchestration produces a different piece to the ear. Across a catalogue, fixed instrumentation is a powerful sameness source — arguably the *most immediately audible* one, since timbre is registered pre-attentively and instantly.
- **Register and spacing.** Where material sits (high/low), and how voices are spaced (close vs. open, drop voicings). Register arc over time (rising to a climax, etc.) is part of the dynamic/textural shape.
- **Dynamic and textural arc.** The trajectory of intensity across the piece — a build, a climax, a release. A piece with a *static homogeneous texture* has no arc; a piece whose texture *develops* has a narrative. This is the textural analogue of Axis 1's develop-vs-restate distinction.

The diagnostic pivot: **a static homogeneous texture makes every section — and every piece — sound alike; a developing texture gives each piece a distinct arc even from shared materials.** Timbre and texture are also the *fastest-registering* difference for a naive listener, so texture sameness is often the first thing consciously noticed even when the deeper cause is rhythmic or formal.

### Diagnostic test

- **IMPLEMENTED:** Do instrumentation, voice-role assignment, register, and texture density vary between pieces, *and* does texture develop *within* a piece (sectional thinning/thickening, homophonic↔linear shifts, a dynamic/textural arc toward a climax)?
- **SHALLOW-FAKED:** Is there a single fixed ensemble and a single fixed role layout (e.g., always the same lead patch over the same pad over the same bass) with only *notes* changing? Is "arrangement variety" limited to swapping a timbre while the role structure, density, and static full-texture-throughout remain identical? A tell: no concept of sectional textural contrast — the texture at bar 1 equals the texture at the climax equals the texture at the end.
- **ABSENT:** Is there no arrangement layer at all — a single monophonic or fixed-block realization with no roles, no register planning, no arc?

### Contribution to a "sameness" verdict

Texture sameness is the **most immediately *noticeable*** contributor (timbre registers first) but not necessarily the *deepest cause*; a rich arrangement over a fixed groove-and-form template will still eventually sound same-y, while a plain arrangement over varied grooves and forms will sound more distinct than its thin orchestration suggests. For a complaint about *rhythmic feel* specifically, arrangement is a secondary contributor — unless the fixed texture includes a fixed rhythmic accompaniment pattern, in which case it overlaps directly with Axis 4 (the groove skeleton often *lives in* the arrangement layer).

---

## Synthesis: Ranking and the Single-Architecture Hypothesis

### Ranking the five axes by likely contribution to cross-piece sameness

For the specific reported complaint — "sameness, and the same common rhythm-feel across every piece" — the axes rank as follows. The ranking is driven by one principle: **the axes that define the temporal *container* dominate the axes that define the *content*, because the container is what the ear tracks as the identity of the piece, and re-rolling content inside a fixed container cannot change perceived identity.**

1. **Axis 4 — Groove / Feel / Rhythm. (Leading cause.)** The listener named rhythmic feel. A fixed subdivision grid, swing/straight setting, meter, and groove skeleton constitute an invariant rhythmic *surface*, and the parameters most engines expose (tempo, note placement, cadence location) are all *content inside that surface*. This is the direct, literature-supported explanation for "varying tempo doesn't fix it."

2. **Axis 3 — Form / Harmonic-Rhythm Variety. (Co-primary, tightly coupled to Axis 4.)** A fixed phrase grid, single form, and constant harmonic-rhythm rate are the *other half* of the fixed temporal container. Axes 3 and 4 together are "the stamped template." They are ranked just below Axis 4 only because the complaint's wording points at rhythm-feel specifically; mechanistically they are one problem — a single fixed temporal architecture — and should be treated together.

3. **Axis 5 — Arrangement / Orchestration Arc. (Most *noticeable*, mid-depth cause.)** Fixed instrumentation and static texture are registered fastest by the ear and strongly reinforce sameness, and when the fixed texture carries a fixed accompaniment rhythm it merges into Axis 4. Ranked here because texture variety alone cannot rescue a fixed groove/form, but its absence loudly advertises the sameness.

4. **Axis 2 — Harmonic Vocabulary. (Strong global-sameness cause, weak rhythm-specific cause.)** A small fixed diatonic vocabulary makes every piece harmonically related, but harmonic poverty is not what a listener means by "same rhythm." High contributor to a general "these all sound alike," low contributor to the *rhythmic* framing.

5. **Axis 1 — Melodic Development. (Compounding factor.)** Weak development produces "it never goes anywhere" within a piece and rhyming melodic surfaces across pieces, but is the least likely to be *named* as rhythmic sameness. It compounds the others rather than leading.

The ranking is not an instruction to fix only Axis 4. It is a claim about *where the perceived sameness originates*: in the fixed temporal container (Axes 4 and 3), amplified by fixed texture (Axis 5), on a base of limited harmonic and melodic vocabulary (Axes 2 and 1).

### The single-architecture hypothesis

**Hypothesis under test:** *Per-parameter variety on a single fixed architecture — one phrase template, a small progression set, one fixed rhythmic feel, and motifs that repeat rather than develop — cannot fix a same-architecture sameness problem; the architecture itself must gain a developmental / vocabulary dimension.*

**Verdict: the literature supports this hypothesis, and it is the correct diagnosis rather than a wish list.** The reasoning is consistent across all five axes and reduces to a single principle already stated above:

- **Perceived musical identity is carried by structure (the container), not by parameter values (the content).** Schoenberg's develop-vs-restate distinction (Axis 1), the reharmonization principle (Axis 2), the phrase/form/harmonic-rhythm-as-container principle (Axis 3), the tempo-vs-feel distinction (Axis 4), and the develop-vs-static-texture distinction (Axis 5) are five statements of the same fact: **changing values on a fixed structure changes the notes but not the felt identity, because the ear categorizes by structure.**

- Therefore, re-rolling parameters inside one architecture moves each piece *within a single perceptual equivalence class*. The pieces differ measurably (different notes, chords, tempos) yet cluster tightly perceptually, which is exactly the operational definition of "they all sound the same." No density of parameter variation escapes the class, because the class boundary is the architecture, not the parameters.

- The fix the literature prescribes is categorical, not quantitative: give the system **plural structures** where it currently has one — a **vocabulary of grooves/feels** (multiple subdivision grids, swing/straight, meters, groove skeletons), a **vocabulary of forms and phrase schemes**, variable **harmonic-rhythm rates**, a genuinely expanded **harmonic vocabulary** (secondary function, modal interchange, tensions, modulation), and true **motivic development** (transformation, not restatement) plus a **developing texture**. Each of these adds a *dimension* (a new axis of between-piece difference), which is what multiplies the number of perceptual equivalence classes the system can occupy. Parameter tuning adds *resolution within a class*; architectural pluralization adds *classes*. Only the latter changes the between-piece distance that the "sameness" verdict measures.

**Adversarial check.** The hypothesis must not be over-read into "any one architectural addition fixes sameness." It does not. Adding a second groove but keeping one form and one harmonic language still yields a small, quickly-exhausted set of classes; the ear will find the new sameness. The correct reading is directional and cumulative: sameness is escaped by **adding developmental/vocabulary *dimensions* to the architecture**, and the marginal return is largest on the axes that define the temporal container (Axis 4, then Axis 3), because those classes dominate perception. A single new dimension helps; durable distinctiveness requires several, coherently combined. The failure mode to avoid is *uniform* application of any new vocabulary (every chord a 13th, every piece medium-syncopated, every texture through-composed), which merely relocates the constant — the goal is *plural and selected*, not *uniformly maximal*.

---

## Citations

Authoritative pedagogy and primary theory are cited by author/title; where a claim rests on a weaker or secondary source it is flagged inline in the text.

- Schoenberg, Arnold. *Fundamentals of Musical Composition.* Ed. Gerald Strang & Leonard Stein. Faber, 1967. — motive, developing variation, repetition vs. variation, liquidation, fragmentation. Public scan: archive.org, "A. Schoenberg — Fundamentals of Musical Composition." https://ia903208.us.archive.org/1/items/MusicTheoryGeorgeThaddeusJones1974/A.schoenberg-FundamentalsOfMusicalComposition_text.pdf
- Boss, Jack. "Motivic Processes in Schoenberg's op. 11, no. 3." *Music Theory Online* 21.3 (2015). — corroborates Schoenberg's developing-variation and liquidation terminology. https://mtosmt.org/issues/mto.15.21.3/mto.15.21.3.boss.html
- Perricone, Jack. *Melody in Songwriting: Tools and Techniques for Writing Hit Songs.* Berklee Press, 2000. — motif, sequence, variation, extension/inversion in the Berklee melodic-writing lineage. https://berkleepress.com/songwriting/melody-in-songwriting/
- Nettles, Barrie & Richard Graf. *The Chord Scale Theory & Jazz Harmony.* Advance Music, 1997. — Berklee chord-scale harmony: diatonic function, secondary/special-function dominants, modal interchange, tensions, modulation, nonfunctional harmony. https://www.alfred.com/the-chord-scale-theory-jazz-harmony/p/01-ADV11216/
- Pease, Ted & Ken Pullig. *Modern Jazz Voicings: Arranging for Small and Medium Ensembles.* Berklee Press. — voice roles, voicings, texture, contrapuntal/linear vs. block textures. https://berkleepress.com/berklee-authors/ted-pease/
- Pease, Ted. *Jazz Composition: Theory and Practice.* Berklee Press, 2003. — texture, dynamics, sectional contrast, arrangement arc. https://berkleepress.com/berklee-authors/ted-pease/
- "Harmonic rhythm." *Wikipedia* (tertiary, used only for the standard definition "rate at which chords change" and the acceleration-toward-cadence convention, both of which are common-practice consensus). https://en.wikipedia.org/wiki/Harmonic_rhythm — and the pedagogical tutorial: "Harmonic Rhythm," *Comprehensive Musicianship, A Practical Resource*, Iowa State Univ. Pressbooks. https://iastate.pressbooks.pub/comprehensivemusicianship/chapter/6-4-harmonic-rhythm-tutorial/
- Witek, Maria A. G., Eric F. Clarke, Mikkel Wallentin, Morten L. Kringelbach & Peter Vuust. "Syncopation, Body-Movement and Pleasure in Groove Music." *PLoS ONE* 9(4): e94446, 2014. — the inverted-U relationship: medium syncopation maximizes pleasure and desire to move. https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0094446
- Datseris, G. et al. / Corcoran et al.; and "Microtiming Deviations and Swing Feel in Jazz," *Scientific Reports* 9, 19824 (2019). — swing as a systematic long–short subdivision ratio; groove correlating more with density/salience/syncopation than with microtiming magnitude. https://www.nature.com/articles/s41598-019-55981-3
- Frühauf, Kopiez & Platz, and Madison et al.; summarized via "The Effect of Expert Performance Microtiming on Listeners' Experience of Groove," PMC (2016) — groove defined as a temporal gestalt of pulse and subdivision; microtiming is not the primary groove lever. https://pmc.ncbi.nlm.nih.gov/articles/PMC5050221/

*Note on source strength:* The Berklee Press and Schoenberg texts are the authoritative pedagogy and are relied on for the compositional principles. The music-perception papers (Witek et al.; the *Scientific Reports* swing study; the groove/microtiming reviews) are peer-reviewed and are relied on specifically for the empirical claims about syncopation, swing ratio, subdivision, and density as the perceptual carriers of groove distinctiveness. The single tertiary source (Wikipedia) is used only for a definition that is uncontested common-practice consensus and is corroborated by the Iowa State pedagogical tutorial.

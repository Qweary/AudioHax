# S44 Aesthetics Design — Variety as a Form-Deployed Device (texture-arc, not raw thickness)

DESIGN / ASSESSMENT DOCUMENT. No source edited. This is the composition &
songwriting-aesthetics lens on the operator's forward-flagged S43 arc: **deepen
the per-layer / per-voice musical VOCABULARY** — read as two signals that are one
problem: (1) more variety at *every* layer; (2) support `>4` instrument lines as
a future feature.

The lens: what per-layer variety and voice count serve the AESTHETIC EXPERIENCE
— form, contrast, departure-and-return, payoff, memorability — and where naive
multiplication HURTS the listen. This is NOT chord internals (Music Theory) and
NOT pixel features (extraction). It answers: given that the engine *can* add
voices and vary layers, **how should it deploy them so the listen is more
pleasing, not just thicker.**

The trained-ear gate is the standard: the operator hears when a texture is
shapeless, when a B section fails to contrast, when a climax is unearned, and
when a piece runs full-tutti from bar one and has no dynamic range left to give.

---

## 0. The binding frame, restated as the design's spine

The lead's load-bearing counterpoint governs this whole document, so it is the
first thing stated, not a caveat at the end:

> **Raw voice count is cheap but aesthetically EMPTY without per-voice
> differentiation and without DEPLOYMENT IN TIME.** `">N voices meaningfully"`
> is a FACET of `"more variety"`. A piece that runs all N voices flat-out for
> the whole duration has no dynamic range to give. Variety is most pleasing when
> it is DEPLOYED ACROSS THE FORM — texture as a contrast/arc device (sparse
> Statement → fuller Contrast → resolved/climactic Return) — not when it is
> maxed everywhere. Adding voices without an arc to deploy them in is the
> thin-multiplication trap in aesthetic terms: **texture with no shape.**

Two grounded facts from the code make this frame concrete and falsifiable, and
they are why this design exists:

1. **Texture is selected ONCE per plan and cloned identically onto every
   section.** `CompositionPlanner` (`src/composition.rs:1513–1521`) picks one
   `OrchestrationProfile` from the `texture` SelectTable over the *whole-image*
   knobs, then `orchestration.clone()`s the same profile onto every `Section`
   (`:1625`). The Statement, the Contrast, and the Return all carry the **same
   layer set, the same `pad_voices`, the same figuration.** The code comment at
   `:1515` is explicit: *"section-conditioned selection is a later slice."* So
   today, however many voices a piece has, it has that many voices *the whole
   time*. There is **no texture arc.** This is the flat-texture-with-no-shape
   state the binding frame warns against, already in the tree.

2. **A per-section DENSITY spine already exists.** `Section.density`
   (`:1606`) IS computed per-section from source-region energy. So the engine
   already has a per-section scalar that *could* carry a texture arc — the
   machinery for "vary across the form" is half-built (density varies; the
   voice-set and the figuration do not). This is the seam the texture arc lands
   in, and it means the arc is an *extension of an existing per-section field*,
   not a new architecture.

The aesthetic conclusion that organizes everything below: **the next variety win
is not "add a 5th voice" and it is not "add more texture profiles." It is to
make the texture the piece already has be DEPLOYED across the section roles, so
the listener hears departure-and-return in the FABRIC, not only in the harmony.**
Voice count `>4` is real and worth doing — but it earns its keep only once there
is a form arc to deploy it into. Voices first need somewhere to *arrive*.

---

## 1. VARIETY AS A FORM-DEPLOYED DEVICE — texture thinning/thickening bound to section roles

### 1.1 The pleasing-shape rationale

The single most reliable source of satisfaction in a short piece is
**departure-and-return**: state home, leave, come back, make the return feel
EARNED (the form knowledge base, and every prior aesthetics doc S24/S26/S42).
Harmony and key plan are one way to dramatize that arc. **Texture is the other,
and it is the one the engine is currently wasting.** Orchestration density —
how many voices sound, how active they are — is a primary dramatic lever in all
tonal music: the verse is sparse so the chorus can bloom; the development thins
so the recapitulation can land full; the coda strips back to one voice so the
silence after means something. A listener feels a return *land* not only because
the tune came home but because the FABRIC swelled to meet it.

Today the engine throws this away by running one texture flat across the whole
form. A piece that is `pad_figured` (4 layers, animated bed) in its Statement is
*also* `pad_figured` in its Contrast and its Return. The B section that should
provide relief sounds exactly as full as the A it is supposed to contrast with;
the Return that should bloom has nothing extra to bloom *into* because it was
already maxed in bar one. **The piece has texture but no texture SHAPE** — which
is the binding frame's "texture with no shape," verbatim, present in the tree.

### 1.2 The role→texture binding (the design)

Variety — per-layer AND voice count — should be bound to the section's
`ThematicRole` (`Statement / Contrast / Return / Development / Coda`,
`composition.rs:406`), not applied uniformly. The aesthetic target arc, stated
as a texture plan keyed on role:

| Role | Texture posture | Voices | Activity | Why it is more pleasing |
|---|---|---|---|---|
| **Statement (A)** | establish, room to grow | medium (e.g. melody + pad + bass) | moderate | The ear must LEARN the material before it can be developed; a Statement that is already full has spent its dynamic range and leaves the Return nowhere to go. Sparser A = the Return can *add*. |
| **Contrast (B)** | DIFFERENT, not just more | thinner OR re-colored (drop the pad to a pedal; or drop the bass and float the upper voices) | lower or differently-distributed | B earns the name "Contrast" by being audibly UNLIKE A. Texture change is the cheapest, most reliable way to make B contrast — and it gives B's thinner harmony somewhere to breathe instead of reading as a hole. |
| **Return (A′)** | FULLEST — the texture climax | most (add the counter-melody / the 5th voice HERE) | highest | The return is where added voices PAY OFF. A counter-melody or an extra line that enters *only* at A′ makes the homecoming feel like an arrival, not a repeat. This is the single best home for `>N` voices. |
| **Development** | building, accumulating | rising | rising | If present, Development is the on-ramp to the climax: voices/activity ratchet UP toward the Return so the fullness is *earned* by accumulation, not switched on. |
| **Coda** | strip back, settle | fewest (often melody + bass, or a single pedal) | low, decaying | A coda that thins to one or two voices makes the ending land and the final silence mean something. Full-tutti to the last note has no release. |

The throughline: **sparse Statement → contrasting (often thinner) B → fullest
Return → stripped Coda.** That is a texture arc. It is the same departure-and-
return shape the form already encodes harmonically, now also carried by the
fabric — so the two reinforce instead of the texture sitting flat under a moving
harmony.

### 1.3 Why this is the binding-frame answer, not a workaround

This binding is *exactly* what "deploy variety in time" means operationally. It
converts the operator's "more variety at every layer" from "add more stuff
everywhere" (the trap) into "let the layers the piece already has RISE AND FALL
across the sections" (the win). It is also nearly free in mechanism terms: the
per-section `density` field already exists and already varies; the missing piece
is letting the *orchestration profile / voice-set* be selected per-section-role
the same way density already is, rather than cloned flat. The aesthetic content
of this design is the ARC SHAPE (the table above); the mechanism is "select
texture per section role instead of once per plan," which the code comment at
`:1515` already names as the planned next slice.

---

## 2. THE AESTHETIC CASE FOR VARIETY-FIRST — why texture-arc is the next bottleneck after S43

### 2.1 What S43 fixed, and what it left

S43 foregrounded the melody (the `melody_forward` default + the two-tier
`subject_melody` escalation). Per the S43 taste verdict that is real: `example`
now has a followable hook; the over-loud inner Fill was demoted; the figure/
ground inversion is fixed. **The piece now has a clear SINGER.** That was the
right first move — identity comes from the foreground, and the foreground was
buried.

But a clear singer over a **thin, undifferentiated, uniform-texture bed** is the
next ceiling. S43 made the melody audible *as a figure*; it did nothing about the
GROUND the figure sits on. And the ground is flat: same 3-or-4 layers, same
figuration, same density-of-fabric, Statement through Return. A great hook over a
texture that never changes is a song that says its one thing and then says it
again at the same dynamic — pleasant, but shapeless past the first phrase. The
S42/S43 lineage solved "where is the identity" (the foreground). S44 is the next
question in that exact sequence: **"the foreground is clear — now does the piece
have a SHAPE over its duration, or is it the same fabric the whole way?"**

### 2.2 Why texture-arc beats raw thickness (the falsifiable claim)

The naive read of "more variety / more voices" is *thicker*: add a 5th line, add
busier figuration, run it all the time. This is the same trap S42 diagnosed for
bed-variation, in a new costume. Three reasons texture-arc beats raw thickness,
each falsifiable by listening:

1. **A constant full texture is as monotonous as a constant thin one** (the S16
   texture design says this outright). Monotony is not a function of how *much*
   is sounding — it is a function of how much the amount *changes*. A piece that
   is 5-voice-full from bar one to the last bar has zero textural dynamic range;
   it cannot make a Return feel like an arrival because it was already there.
   *Falsifiable:* if you add a 5th voice everywhere and the piece sounds busier
   but not more SHAPED — no point where it lifts, no point where it relaxes —
   thickness lost.

2. **Contrast requires a reference level.** A B section can only sound like
   relief, and an A′ can only sound like a bloom, RELATIVE to a Statement that
   was different. Maxing every section destroys the reference — there is nothing
   for the contrast to be measured against. Thickness everywhere is
   self-cancelling: it removes the very baseline that would let added voices
   register as "more."

3. **Mud.** In this narrow, mostly-diatonic harmonic language, more than ~4–5
   simultaneous pitches in the same register span muds fast (S16 §2.1). Naive
   thickening crowds the octaves and turns distinct lines into a wash — the exact
   "ethereal / structureless" complaint from S13 that opened the composition-
   architecture arc. Deployed thickening (voices enter for the climax, then
   leave) keeps the average texture clear and spends the density only where it
   pays.

So the aesthetic ordering after S43 is: **texture-arc (shape the fabric across
the form) BEFORE raw thickness (add voices everywhere).** Adding voices is the
*next* lever, and it slots cleanly into the arc once the arc exists (§3, §5) —
but adding it first, with no arc, re-commits the thin-multiplication trap with a
bigger number.

---

## 3. WHEN `>N` VOICES EARNS ITS KEEP AESTHETICALLY

The future `>4`-voice feature is real and worth building. The aesthetic
discipline is *where* the extra lines sound, not *that* they exist.

### 3.1 Which images / forms justify more lines

- **Panoramic / high-energy / busy images → fuller climax justified.** An image
  with high `foreground_energy` / `background_energy` / `arousal` (the saliency
  + affect knobs, `composition.rs:794–807`) is reading as "much going on,
  energetic, full." A fuller texture at its Return/climax is the music tracking
  the image's energy. These images can carry a 5th (counter-melody) or even a
  6th (ambient pedal) line — *at the climax.* A landscape that fills the frame,
  a crowd scene, a bright high-activity abstract: these EARN the extra lines.
- **Forms with a genuine climax point justify it more.** `ABBA`/`ABBAC` (B
  intensified before the A return), `rondo` (refrains that should each feel
  fuller), `theme-and-variations` (each variation can add a layer) — these forms
  have a structural place for an accumulating voice. `Development → Return` arcs
  are the textbook on-ramp. The extra voice enters *there.*
- **Sparse / intimate / low-arousal / dark images → more pleasing SPARSE.** A
  `Nocturne`/`Drone`/`Lament`-leaning image (low `arousal`, dark `value_key`,
  cool warmth) is reading as still, intimate, somber. Piling voices onto it
  *fights the affect* — it is the key-plan-brightens-a-sad-image error in the
  texture dimension. These images should run FEW voices and let the space be the
  point. A 2–3 voice nocturne with a clear melody over a held pad is more
  pleasing than a 5-voice one; the extra lines are noise against the mood.

The Affect lens (§6) owns the per-affect-state voice ceiling; the binding here is
that the **voice budget is itself an image-conditioned variety dimension**, not a
constant — a bright panorama and a dark intimate portrait should not get the same
number of lines, and CERTAINLY should not both get the maximum everywhere.

### 3.2 The orchestration arc that makes added voices a payoff, not mud

Added voices are a payoff iff they are DEPLOYED, differentiated, and registrally
separated:

- **Deploy them in time:** the extra line(s) enter at the Return/climax (or
  accumulate across a Development), and are absent in the Statement. An extra
  voice that is present everywhere is back to the flat trap. An extra voice that
  *enters for the homecoming* is an arrival.
- **Differentiate them:** a "voice" that doubles an existing line's rhythm an
  octave away is not a new voice aesthetically — it is thickness. A real added
  voice has its own RHYTHMIC PROFILE (it moves when the melody holds, holds when
  the melody moves) so the ear hears it as a *second strand*, not a doubling.
  This is the S16 counter-melody discipline; raw count without it is mud. (The
  *legality* of that independent line — real counterpoint, no parallel perfects —
  is the Music Theory lens's, §6.)
- **Separate them registrally:** keep the lines spread across the register span
  (the S16 "never two voices in the same pitch-class octave; ≥ a third between
  inner voices" rule). Crowding lines into one octave is the fast road to the
  ethereal wash.

The aesthetic test for any added voice: **does the piece sound more SHAPED, or
just busier?** If the listener can point to a moment where the texture blooms and
a moment where it relaxes, the voice earned its keep. If the whole thing just got
denser with no arc, it did not.

---

## 4. GUARD-RAILS — encodable, testable PROPERTIES for "pleasing variety"

Each is a property the generator should not be able to violate, and each is
falsifiable by listening. They are the texture-dimension analogues of the S42/S43
prominence guard-rails and the S24 key-plan invariants.

1. **Texture-actually-varies-across-sections.** Within a piece, the resolved
   per-section texture (voice count and/or density and/or figuration activity) is
   NOT identical for every section — at least one role boundary shows a change.
   *Testable:* assert that `{(section.voice_count, section.density,
   section.figuration_activity)}` over the sections is not a constant set for any
   multi-section form. *Falsifiable by ear:* if the fabric sounds identical
   Statement-to-Return, the arc did not deploy.

2. **Sparseness-floor (not every section maxed).** At least one non-climax
   section sits BELOW the piece's textural maximum — the Statement and/or the
   Contrast are not at full voice count / full density. *Testable:* `min over
   sections of voice_count < max over sections of voice_count` (for any form
   with ≥ 2 distinct roles and a voice budget > the floor). *Falsifiable by ear:*
   if every section is full, there is no relief and no room for the climax.

3. **Variety-serves-contrast (B differs audibly).** A `Contrast` section differs
   from its neighboring `Statement`/`Return` in ≥ 1 of {voice set, density,
   figuration, register distribution} — i.e. B's contrast lands in the FABRIC,
   not only in the harmony. (This generalizes the existing S42 "contrast-
   actually-contrasts" guard from key/mode/density to the full texture.)
   *Testable:* for each `Contrast` section, assert ≥ 1 texture dimension differs
   from the adjacent non-Contrast section. *Falsifiable by ear:* if B sounds like
   "A again in the same clothes," it failed.

4. **No-uniform-tutti-throughout.** No multi-section piece runs the full voice
   budget across ALL sections. The maximum voice count appears in at most the
   climax section(s) (Return / intensified B′ / final variation), never in the
   Statement. *Testable:* assert the Statement's voice count `<` the piece's max
   voice count whenever the budget exceeds the floor. *Falsifiable by ear:* if
   the piece is full-tutti from the first bar, the climax cannot bloom.

5. **Climax-is-earned.** The fullest texture section is a climax role
   (`Return`, intensified `B′`, final variation, or the section a `Development`
   leads into) — NOT the Statement and NOT the Coda. *Testable:* assert
   `argmax(voice_count or density)` over sections has a climax-eligible role.
   *Falsifiable by ear:* the textural high-water-mark should coincide with where
   the piece feels like it arrives; if the densest moment is the opening, the
   shape is inverted.

6. **Coda-recedes (if a Coda exists).** A `Coda` section's texture is at or below
   the Statement's, not above — the ending settles. *Testable:* `coda.voice_count
   ≤ statement.voice_count` (and density ≤). *Falsifiable by ear:* an ending that
   is fuller than the opening does not feel like an ending.

7. **Bed-never-vanishes / melody-stays-foreground (carry-forward).** The S43
   foreground guard-rails STILL hold under any texture arc: resolved Melody
   prominence `> 0.5` on every section; bed roles recede but stay `> 0.25`. The
   texture arc may THIN the bed in B, but it may not erase the foreground or
   hollow the ground to nothing. *Testable:* the S43 invariants, re-run on every
   section of an arc'd piece. (This is the guard against "thin B" overshooting
   into "empty B.")

8. **Legacy/identity path byte-frozen.** The `identity` orchestration profile
   (`pad_voices: 0`, empty layers) and the legacy render path are unchanged;
   `src/engine.rs` sha256 stays `e50c7db1…2348261`. The arc is a *compose-path*
   feature; the identity/legacy plan still clones one flat (identity) profile and
   stays byte-stable. *Testable:* the existing byte-freeze test still passes.

---

## 5. SLICEABILITY

### 5.1 The single most pleasing audible variety win for S45 (v1-essential)

**Bind the orchestration profile to section ROLE so the Return is fuller than the
Statement and the Contrast is thinner/different — i.e. make the texture arc real
with the voice budget the piece ALREADY has (≤ 4 voices), before adding any new
ones.**

Why this is the v1-essential win:

- It directly cashes the binding frame: it deploys variety *in time* using the
  fabric already present, which is the prerequisite for everything else and the
  highest-leverage single change.
- It is the change the code already names as next (`:1515` "section-conditioned
  selection is a later slice"). The seam exists: instead of selecting one
  `OrchestrationProfile` per plan and cloning it (`:1517–1521, :1625`), select
  per section keyed on `thematic_role` (a sparser profile for `Statement`, a
  thinner/re-colored one for `Contrast`, the fullest for `Return`). The
  per-section `density` field (`:1606`) already proves per-section variation is
  architecturally fine; this extends the same per-section treatment to the
  voice-set/figuration.
- It is freeze-safe in shape: identity-path plans still resolve to the one
  identity profile (byte-stable); only the non-identity compose path gains the
  per-role selection.
- The audible result the operator should hear: a piece that **opens with room,
  contrasts in its middle, and BLOOMS at the return** — departure-and-return in
  the fabric, not just the harmony. That is a shape, where before there was a
  flat bed.

Concretely for the implementer (the seam, not the realizer internals which are
Music Theory / Architect): the `texture` SelectTable selection at `:1517` becomes
role-aware — either (a) a per-role default ladder (Statement→`pad_bed`,
Contrast→a thinner profile e.g. a pad-pedal or bass-dropped variant,
Return→`pad_figured`/`pad_bed_counter`), modulated by the image's overall energy;
or (b) the image picks a *target* (climax) profile and the planner DERIVES the
sparser Statement/Contrast profiles from it (e.g. drop `pad_voices` by 1 and
simplify figuration for non-climax roles). Option (b) keeps the image→texture
mapping the operator already tuned and layers the arc on top, which is the
cleaner aesthetic answer — the image still chooses the piece's textural
character; the form chooses how that character is *deployed.* I recommend (b).

### 5.2 Later refinements (explicitly out of the v1 slice)

- **The `>4`-voice feature (the operator's facet 2).** Build it AFTER the arc
  exists, and slot the extra line into the arc's CLIMAX: a counter-melody / 5th
  line that enters only at the `Return` (or accumulates across a `Development`),
  absent in the Statement. This is the §3.2 deployment discipline. Building it
  before the arc re-commits the flat trap; building it into the arc makes every
  added voice a payoff. The voice budget should itself be image-conditioned
  (panoramic/high-energy → up to 5–6 at climax; intimate/dark → capped low),
  §3.1.
- **Per-layer figuration variety across the arc.** Once role-aware texture
  selection exists, vary the *figuration activity* across roles too (Statement: a
  calm block/alberti bed; Return: a more animated broken-chord/stride bed) so the
  bloom is also a rhythmic-activity bloom, not only a voice-count one. Refinement,
  not v1.
- **Phrase-positional micro-arc within a section** (thin at phrase starts, thicken
  into the interior — S16 §2.2). A finer-grained deployment that sits under the
  section-level arc. Defer.
- **Counter-melody as a real independent line** (S16 §2.3) — the differentiated
  voice that makes added count not-mud. This is the content of the `>4` voice; its
  legality is Music Theory's. Pairs with the voice-count slice.

### 5.3 How the N-voice feature slots into the form arc later (the staging picture)

```
S43 (done):  foreground the melody          → the piece has a clear SINGER
S45 (v1):    texture ARC bound to role       → the piece has a SHAPE (sparse A → thin B → full A′),
                                               using the voices it already has
later:       >N voices, deployed at climax    → the bloom gets BIGGER and more differentiated;
                                               extra lines enter at the Return, capped by affect
later:       per-role figuration + phrase arc → the bloom is also rhythmic, finer-grained
```

The ordering is load-bearing: each step needs the previous one to be audible.
Voices added before the arc are thickness; voices added into the arc are payoff.

---

## 6. OPEN TENSIONS for the other lenses

### 6.1 For the Music Theory lens

- **Which added lines need real counterpoint.** The §3.2 / S16 rule is that an
  added voice is aesthetically a *voice* only if it has independent motion
  (contrary/oblique to the melody, fills its gaps). The *aesthetic* requirement
  is "sounds like a second strand"; the *legality* — no parallel perfects between
  melody and the new line across T→T+1, passing tones resolving by step,
  first-species floor — is Theory's. Flag: a 5th line added at the climax with no
  counterpoint discipline will sound like doubling/mud and waste the bloom.
- **Voice-leading under a thinning B.** When the Contrast section DROPS the pad to
  a pedal or drops the bass, the remaining voices must still voice-lead cleanly
  and the harmony must still be complete enough to read. Theory should confirm a
  thinned B doesn't leave the harmony ambiguous (a B that's "melody + pedal" must
  still imply its chords).
- **The climax bloom must not break the cadential homecoming.** The fullest
  texture lands at the Return — which is also where the structural PAC lands. The
  added voices/activity must not bury or contradict the cadence; the homecoming
  must still be heard (carry-forward of the S43 cadence watch-item, now under a
  fuller texture).

### 6.2 For the Affect lens

- **Which affect states call for fuller vs sparser — the voice ceiling per
  affect.** §3.1 sketches it (panoramic/high-arousal → fuller climax; intimate/
  dark/low-arousal → sparser, capped low) but Affect owns the actual per-state
  voice budget and the climax-fullness curve. The binding constraint: **the
  texture arc must reinforce the affect, never fight it** — a sad/still image
  should get a SMALL arc (it never blooms to full tutti; its "climax" might be a
  third voice, not a sixth), and a bright energetic image gets a big one. A flat
  max-voice texture is the affect-blind failure in the texture dimension, exactly
  as a key-plan that brightens a sad image is in the harmony dimension.
- **Does the Contrast's thinning read as the RIGHT kind of contrast for the
  affect?** Thinning B can read as "relief/intimacy" (good for a warm image) or
  as "hollowing/desolation" (right for a lament, wrong for a romp). Affect should
  steer whether B thins (relief) or re-colors at equal density (mood-shift), per
  the image's emotional character.

### 6.3 For the Rust Architect

- **Where the form→texture binding lands.** The seam is the per-plan texture
  selection at `composition.rs:1517–1521` and the clone at `:1625`. The arc means
  selecting/deriving an `OrchestrationProfile` per section keyed on
  `thematic_role`, instead of cloning one flat profile. The per-section `density`
  field (`:1606`) is the precedent that per-section texture variation is
  architecturally clean. Decision for the Architect: does the per-role profile
  come from (a) a role-keyed ladder in the `texture` SelectTable, or (b) a
  planner-side DERIVATION from the image-selected climax profile (drop a voice /
  simplify figuration for non-climax roles)? §5.1 recommends (b) on aesthetic
  grounds (the image keeps choosing the textural character; the form deploys it);
  the Architect owns whether (b) is cleaner in the type model or whether the
  ladder (a) is.
- **The `>N`-voice feature's type reach.** Supporting `>4` instrument lines
  touches `num_instruments` (`engine.rs:203`, default 4) and the
  `instrument_role` stratification. The aesthetic constraint to carry into that
  design: the voice budget is per-section (climax sections get more), not a
  global constant — so the Architect should confirm the role-assignment can be
  per-section, not fixed per-piece, OR that voices can be muted per-section (a
  voice present in the instrument list but RESTING in the Statement, entering at
  the Return). The latter (a fixed instrument set, per-section masking) may be the
  freeze-safer path — flag for the Architect to weigh against changing
  `num_instruments` semantics.
- **Freeze:** every part of this is a compose-path / planner change; the identity
  profile and the legacy render path stay byte-frozen (`engine.rs` sha256
  unchanged). The arc must be inert on the identity path (one flat identity
  profile, no per-role selection).

---

## Appendix — no `mappings.json` rows authored in this design

This is a design/assessment document. It does NOT author `mappings.json` rows: the
v1 slice is primarily a PLANNER seam change (role-aware texture selection at
`composition.rs:1517`), and any new texture-profile rows it eventually needs
(e.g. a thinner Contrast profile, a richer climax profile) should be authored in
the BUILD slice once the Architect fixes the selection seam, so the rows match the
chosen mechanism (ladder vs derivation). When that slice authors rows, they go
through the single-writer coordination with the Music Theory lens (shared
`mappings.json`) exactly as S42 did. Flagged here so the lead routes the row
authorship to the BUILD slice, not this design.

---

*End of S44 aesthetics design. Design-only: no source, test, or asset modified.
Types verified against `src/composition.rs` (ThematicRole `:406`, SectionTemplate
`:750`, FormSpec `:768`, Section.density `:1606`, OrchestrationProfile `:487`,
the per-plan texture selection+clone `:1513–1625`) and `assets/mappings.json`
(texture_catalogue / texture SelectTable / prominence_catalogue). The frozen
`src/engine.rs` is untouched and not proposed for edit.*

# Pattern-Library Research — Musical Patterns for a Deterministic Composer

Date: 2026-06-17
Scope: Music-theory pattern types stated as **encodable rules and data** for a pure-Rust, heuristic, no-ML image→music engine.
Posture: Theory and TYPES only. No transcriptions of any copyrighted composition. Every pattern below is public-domain common-practice theory (the rule-lists a textbook publishes), curated as bounded data — not a song database.

This briefing is written so a downstream architect can encode each finding as deterministic rules + data tables + scored predicates. Each rule is marked **HARD** (always enforce; a boolean gate) or **PREF** (a weighted score term). Where a rule is predicate-shaped it is stated as a function over pitch sequences.

The four areas in priority order:

1. Species counterpoint — the genuinely new capability (an independent melodic voice the engine has never had).
2. Idiomatic chord progressions — catalogue-deepening (new progression rows).
3. Accompaniment / background idioms — catalogue-deepening (new figuration/bass rows).
4. Dissonance-as-intention — research-only groundwork (BUILD deferred).

A note on what already exists, so this targets real gaps: the engine already has a voice-leading layer with `Bass`/`Upper` voice roles, a perfect-fifth upper-voice motion cap, common-tone retention, a HARD `has_parallel_perfects` reject over voice pairs, an `interval_class` helper, a partial `CounterMelody` realizer with a contrary-motion bonus and a unison-double penalty, a phrase/cadence model (`PhrasePosition`: PhraseStart / Interior / HalfCadence / PerfectAuthenticCadence), and a homophonic block/Alberti texture set. The findings below extend that vocabulary; they do not reinvent it.

---

## Terminology Map

- **Cantus firmus (CF)** — a fixed given line. In species pedagogy the counterpoint is composed against it. In this engine the analogue is the existing melody (or the bass): the new line is composed *against an already-realized voice*.
- **Consonance / dissonance** — a classification of the *harmonic* interval between two simultaneously sounding notes. Common-practice classes: **perfect consonances** = unison/octave (interval-class 0) and perfect fifth (ic 7); **imperfect consonances** = major/minor third (ic 3, 4) and major/minor sixth (ic 8, 9); **dissonances** = all seconds (ic 1, 2), all sevenths (ic 10, 11), the tritone (ic 6), and any augmented/diminished interval. The **perfect fourth (ic 5)** is contested (see Contested Areas) — treat as dissonant between two independent contrapuntal voices.
- **Melodic motion** — *step* (≤2 semitones, conjunct) vs *leap* (≥3 semitones, disjunct). Distinct from harmonic interval class above.
- **Relative motion of two voices** — the four types, in the standard preference order: **contrary** (voices move in opposite directions — most independent, most preferred), **oblique** (one voice holds, the other moves), **similar** (same direction, different intervals), **parallel** (same direction, same interval — least independent, restricted). The engine's `MotionDir`/`motion_dir` already classifies single-line direction; relative motion is the pairwise comparison of two such directions.
- **Non-chord tone (NCT) / embellishing tone** — a melodic note that is *not* a member of the sounding chord; it creates a controlled dissonance that resolves. Taxonomy in Area 4.
- **Cadence formula** — a fixed closing two-voice interval pattern (the engine already names HalfCadence and PerfectAuthenticCadence at phrase boundaries).
- **Suspension** — a three-stage NCT: **preparation** (the note is a consonance), **suspension** (it is held while the harmony changes, becoming dissonant on a strong beat), **resolution** (it steps down to a consonance).

---

## Area 1 — Species Counterpoint (HIGHEST VALUE: the independent voice)

Species counterpoint is the Fux-lineage (Gradus ad Parnassum) system for composing one or more independent melodic lines against a given line. It is **the** common-practice formalization of "what makes a second voice an independent voice rather than a parallel shadow," and it is already a rule system — every rule is a checkable constraint over two pitch sequences. The five species are graded by rhythmic density of the new voice against the CF; the rules accrete across species.

The architect should think of this as: given the existing melody (or bass) as the fixed line, generate a second line note-by-note by (a) enumerating candidate pitches, (b) HARD-rejecting any candidate that violates an invariant, (c) scoring survivors by PREF terms, (d) picking the best. This is exactly the shape of the engine's existing `CounterMelody` scorer — these rules deepen it.

### 1.1 Consonance/dissonance classification (foundation predicate)

`fn harmonic_class(a: u8, b: u8) -> Class` over two MIDI notes, via `interval_class(a,b)`:

| interval-class (|a−b| mod 12) | class |
|---|---|
| 0 | perfect consonance (unison/octave) |
| 7 | perfect consonance (fifth) |
| 3, 4 | imperfect consonance (third) |
| 8, 9 | imperfect consonance (sixth) |
| 1, 2, 10, 11 | dissonance (2nd/7th) |
| 6 | dissonance (tritone) |
| 5 | perfect fourth — **contested**; treat as dissonance between independent voices |

This is a total function; no contested edge except ic 5.

### 1.2 First species (note-against-note) — the invariant core

The new note sounds simultaneously with each CF note; one-to-one. Rules (these are the bedrock; higher species inherit them on their *structural* notes):

- **HARD — only consonances vertically.** Every vertical interval between the two voices must be a consonance (perfect or imperfect). No dissonance at all in first species. Predicate: `harmonic_class(new, cf) != Dissonance`.
- **HARD — no parallel perfect consonances.** Two voices must not move from one perfect consonance to the same perfect consonance in similar/parallel motion (parallel fifths, parallel octaves/unisons). The engine's `has_parallel_perfects` already encodes exactly this (same perfect ic at T and T+1 with both voices moving). Keep it.
- **HARD — approach a perfect consonance by contrary or oblique motion** (the "no *direct* / *hidden* fifths/octaves" rule, also called no similar-motion *into* a perfect interval). Stated as a predicate over (prev_pair, this_pair): if `harmonic_class(this) == PerfectConsonance` then the relative motion of the two voices from prev to this must be `Contrary` or `Oblique`, never `Similar`/`Parallel`. (Some pedagogies relax "hidden" perfects when the upper voice moves by step — see Contested Areas; the safe default is to forbid all similar motion into a perfect.)
- **PREF — prefer contrary > oblique > similar > parallel** for every voice-pair transition. The engine already has `CONTRARY_BONUS`; generalize it to a graded score (contrary best, parallel worst) rather than a single bonus.
- **PREF — prefer imperfect consonances (3rds/6ths) over perfect ones for interior verticals.** A line of only octaves/fifths sounds empty and risks parallels; thirds and sixths give the richest two-voice sound. Score imperfect consonances above perfect for non-boundary steps.
- **PREF — limit consecutive parallel imperfect consonances.** Parallel 3rds/6ths are *allowed* but more than ~3 in a row reads as a single thickened line, not two voices. Penalize runs beyond a small cap.
- **Melodic motion of the new line (PREF, with one HARD):**
  - PREF prefer stepwise motion; leaps are the exception.
  - PREF **leap recovery**: after a leap (≥ a fourth), prefer to move by step in the *opposite* direction. After two leaps in the same direction, strongly prefer a step back. (The engine's `motion_dir` gives the direction needed to detect this.)
  - HARD (or strong PREF, by pedagogy) **forbid melodic dissonant leaps**: no augmented intervals (e.g. the tritone, ic 6) or sevenths as a *melodic* interval in the line. The melodic seventh and augmented-second/fourth are the classic prohibited leaps.
  - PREF cap the largest single leap (an octave is the conventional maximum; a sixth is a softer cap). The engine's `MAX_UPPER_VOICE_MOTION = 7` is in this spirit for inner voices; a true melodic counter-line may want a wider but still bounded cap.

### 1.3 Beginning and cadence formulas (HARD boundary rules)

- **HARD — begin on a perfect consonance.** The first vertical interval is a perfect consonance (unison, fifth, or octave). When the counter is *above* the CF, begin on unison/fifth/octave; when *below*, unison/octave (avoid the bare fourth-below opening).
- **HARD — end on a perfect consonance (unison or octave), approached by stepwise contrary motion** — the classic cadence: the two voices converge by step to the octave/unison (the "clausula": e.g. the upper voice's leading-tone steps up while the lower voice steps down, or vice versa). This is the single most rigid formula and maps directly onto the engine's existing `PerfectAuthenticCadence` boundary marker — the counter, at that step, should resolve by step onto the octave/unison with the bass.
- The penultimate vertical interval is typically a **major sixth** (if the counter is above) or a **minor third** (if below), expanding/contracting by step to the final octave. This is encodable as a fixed two-step cadence template keyed off `PhrasePosition::PerfectAuthenticCadence`.

### 1.4 Second species (two notes against one) — dissonance enters, controlled

The new voice has two notes per CF note (a strong/downbeat note and a weak/upbeat note).

- **HARD — the strong (first) note of each pair must be consonant** with the CF.
- **HARD — the weak (second) note may be dissonant only if it is a passing tone**: approached by step and left by step in the *same* direction (it passes between two consonances). Otherwise it too must be consonant. Predicate: a dissonant weak-beat note is legal iff `step_into AND step_out AND same_direction`.
- **PREF — the weak beat may carry a neighbor tone** in some pedagogies (step away and step back). Treat as allowed-by-preference; strict Fux limits second-species dissonance to passing tones only.
- Parallel-perfect checking now applies **across strong beats** (downbeat-to-downbeat), not every note — a parallel fifth on consecutive *downbeats* is still forbidden even if a weak note intervenes. This is the "parallels on the beat" subtlety: encode the parallel check over the strong-beat subsequence, not the raw note stream.

### 1.5 Third species (four notes against one) — passing/neighbor + cambiata

Four new notes per CF note (the engine's `realize_step` already can emit several `NoteEvent`s per step via `offset_ms`, so multiple notes per step is already expressible).

- **HARD — the first note of each group is consonant.**
- **HARD — the middle (2nd/3rd/4th) notes may be dissonant only as stepwise passing or neighbor tones** between consonances.
- **PREF/named-pattern — the *nota cambiata* (changing-note figure):** a specific 5-note licensed dissonance figure: consonance → step down to dissonance → leap down a third → step up → step up (the figure "leaps away from" a dissonance and recovers). It is the one sanctioned place a dissonance is left by leap. Encodable as a fixed template predicate the scorer can *recognize and permit* even though it momentarily breaks "leave dissonance by step."
- **PREF — double neighbor / changing tones** (two NCTs around a chord tone, e.g. upper then lower neighbor) are likewise a licensed figure.

### 1.6 Fourth species (suspensions) — the syncopated voice

The new voice is offset (syncopated) against the CF, producing suspensions.

- **HARD — suspension three-stage shape:** preparation (consonant, on a weak beat) → the note is *tied/held* across the bar so it becomes the strong-beat note → it is now dissonant against the new CF note → **resolution: it must resolve DOWN by step to a consonance.** Predicate over three consecutive states: `prep_consonant AND held_same_pitch AND now_dissonant_on_strong AND next == step_down_to_consonance`.
- **HARD — the resolution is always by step, always downward, always to a consonance** (typically an imperfect consonance: a 3rd or 6th). Standard suspension figures by interval-against-the-bass: **7–6, 4–3, 9–8** (upper-voice); **2–3** (bass suspension). These four are the canonical set; encode them as a small table of (dissonant interval → resolution interval) with the down-step constraint.
- **PREF — a chain of suspensions** (each resolution becomes the next preparation) is the idiomatic deployment; reward consecutive legal suspensions.

### 1.7 Fifth species (florid counterpoint) — the mixture

Fifth species is a free combination of the first four: whole notes, passing tones, neighbor tones, suspensions, and cambiata figures intermixed for a natural-sounding florid line. Encodably, this is **not a new rule set** — it is *selecting among the species-2/3/4 figures per step* under the species-1 invariants. The architect can model fifth species as: at each step, choose a figure (sustain / passing / neighbor / suspension / cambiata) subject to all the HARD gates above, scored by the PREF terms. This is the most musical and the most natural target for an image-driven engine, because the *choice of figure* is exactly the kind of knob image features can drive.

### Area 1 summary of HARD vs PREF
- **HARD (boolean gates, always enforce):** only-consonant verticals on structural notes; no parallel perfect 5ths/8ves; approach perfects by contrary/oblique; dissonance permitted ONLY as a licensed figure (passing / neighbor / suspension-resolving-down / cambiata) with its specific approach+resolution; begin and end on perfect consonances; cadence by stepwise contrary motion to octave/unison; no melodic dissonant leaps (7th, augmented intervals).
- **PREF (scored):** contrary > oblique > similar > parallel; imperfect > perfect for interior verticals; stepwise > leaping melody; leap recovery; cap consecutive parallel imperfects; reward suspension chains and cambiata where licensed.

---

## Area 2 — Idiomatic Chord Progressions (catalogue-deepening)

These are public-domain scale-degree skeletons — the "famous progressions" every theory text lists. Each is encodable as one more hyphenated roman-numeral row in the existing `progression_families` data (warm/cool/neutral), or a small extension if a row needs a voice-leading hint. Stated as scale-degree skeletons with affect and any voice-leading note.

| Name | Scale-degree skeleton | Mode/idiom | Affect | Voice-leading note |
|---|---|---|---|---|
| 50s / doo-wop | `I–vi–IV–V` (loops) | major | nostalgic, sweet, stable | already in catalogue (`I-vi-IV-V`); common-tone-rich, smooth |
| Pop "axis" | `I–V–vi–IV` (and rotations: `vi–IV–I–V`, `IV–I–V–vi`) | major | bright, anthemic, modern | already partly present (`I-V-vi-IV`); the four rotations are one family |
| ii–V–I | `ii–V–I` | major (jazz/common-practice) | strong functional cadence, "home" | already present; ii→V→I is descending-fifths; the 7ths chain smoothly |
| Pachelbel | `I–V–vi–iii–IV–I–IV–V` | major | flowing, processional, ground-bass | already present as neutral row; the bass descends a scale; a classic ground |
| Circle-of-fifths sequence | `I–IV–vii°–iii–vi–ii–V–I` (each root down a fifth / up a fourth) | major | propulsive, "wheel turning," directional | strongest functional drive; voice-lead by keeping common tones and stepping the rest; each link is a descending-fifth |
| Descending-thirds sequence | `I–vi–IV–ii` (roots fall by thirds) or `I–vi–IV–ii–vii°–V` | major | gentle, settling | common-tone between each pair (two shared tones per descending-third) |
| Lament bass (diatonic) | bass `1̂–7̂–6̂–5̂` harmonized, e.g. `i–V6–iv6–V` or `i–VII–VI–V` | minor (Phrygian/Aeolian tetrachord) | grief, pathos, "passus duriusculus" | the BASS is the defining feature: a stepwise descending tetrachord tonic→dominant; encode as a bass-line constraint, not only a chord row |
| Lament bass (chromatic) | bass chromaticized `1̂–♭7̂(7̂)–♮6̂(♭6̂)–5̂…` filling semitones | minor | intensified grief, baroque | same bass-line-driven idea, every semitone filled; voice-lead the chromatic descent in an inner voice |
| Andalusian cadence | `i–♭VII–♭VI–V` (Aeolian) ≡ `iv–III–II–I` (Phrygian) | minor/Phrygian, flamenco | dark, tense, Spanish | a stepwise descending tetrachord in the bass ending on the (major) V; the final V is often a major triad (Picardy-flavored dominant) |
| Phrygian half cadence | `iv6–V` (bass `♭6̂–5̂`) | minor | suspended, archaic, "question" | bass steps down a half-step to the dominant; pairs with the half-cadence boundary the engine already names |
| 12-bar blues skeleton | `I–I–I–I / IV–IV–I–I / V–IV–I–I` (12 bars, three chords) | blues/major (often all dominant-7th quality) | grounded, cyclic, vernacular | a FORM as much as a progression — 12 bars in 3 four-bar phrases; encode as a fixed 12-slot row; chords idiomatically dominant-7th quality even on I and IV |
| Plagal / "Amen" | `IV–I` (or `iv–I`) | major/minor | resolving, hymnal, soft close | a cadence tag; the engine already has a `Plagal` boundary cadence — this is its progression form |
| Deceptive | `V–vi` (major) / `V–VI` (minor) | either | thwarted expectation, "surprise" | the engine already names `Deceptive`; V resolves up by step to vi instead of down to I |

Encodability note for Area 2: most of these are *pure new rows* in `progression_families`. Three (lament bass, Andalusian, 12-bar blues) are **bass-line-defined or form-defined** and benefit from being recognized as a constrained bass descent / fixed-length form rather than only a chord-symbol string — flag for the architect (see Encodability notes). Picking among them is the existing `SelectTable` condition-ladder job (e.g. lament/Andalusian gated to minor/low-valence; blues gated to a "vernacular/groove" feature; axis to high-arousal-major).

---

## Area 3 — Accompaniment / Background Idioms (catalogue-deepening)

Patterns a Bass / HarmonicFill / Pad voice runs *under* a melody. Each is stated in encodable terms: which chord tones, in what order, at what subdivision — directly analogous to the existing `figuration_catalogue` Alberti entry (which already encodes onsets as `{at: fraction-of-step, tone: chord-tone-index}`). The engine's Alberti row uses tone indices `0,2,1,2` = low, high, mid, high, which matches the authoritative Alberti order exactly; the new idioms below follow the same `{at, tone}` schema.

| Idiom | Pattern (chord-tone order @ subdivision) | Register / voice | Idiomatic character / meter |
|---|---|---|---|
| **Alberti bass** | low–high–mid–high, repeating; tone indices `0,3rd-high,5th-mid,3rd-high`; even subdivisions (8ths/16ths) | inner/left-hand fill | classical, elegant, light; any meter; already present |
| **Broken-chord / arpeggiation (ascending)** | root→3rd→5th(→octave) in order, one tone per subdivision, ascending | fill/harp-like | flowing, lyrical; common in 6/8 and 4/4 |
| **Broken-chord (rolling/wave)** | root→5th→3rd→5th or root→3rd→5th→3rd (non-monotone) | fill | gentle motion without an obvious top accent |
| **Block-chord comping** | all chord tones together on chosen beats (e.g. beats 2&4, or every beat) — the engine's `block` figuration with a rhythmic onset pattern | full harmony | pop/jazz comping, hymn; meter-defining |
| **Oom-pah (jump/polka bass)** | beat 1 = bass ROOT (or 5th) low; beats 2 (&/or off-beats) = chord triad higher = "pah" | split bass + mid | polka, march, waltz-lite; duple/triple; root-fifth alternation on strong beats |
| **Stride** | beat 1 & 3 = low bass note (root, or root/5th alternating); beat 2 & 4 = mid-register chord stab | wide-leaping bass + mid | ragtime/jazz piano; 4/4; like oom-pah but wider register leaps |
| **Arpeggiated waltz** | beat 1 = bass root low; beats 2 & 3 = chord upper tones (3rd/5th) higher | bass + fill | 3/4, lilting; the canonical triple-meter accompaniment |
| **Walking bass** | one bass note per beat (quarters), mostly stepwise, outlining chord tones on strong beats with passing/chromatic notes between, aiming the line toward the next chord root | bass only | jazz/blues; 4/4; "tramping" steady feel; uses NCTs (passing) on weak beats |
| **Pedal point** | one sustained (or repeated) bass pitch — usually tonic or dominant — held WHILE the harmony above changes | bass (held) | tension/anticipation, "drone"; any meter; the engine already names a `PedalPoint` ostinato for circular shapes — this is its accompaniment form |

Encodable mechanics:
- Each idiom is a list of `{at: subdivision, tone: chord-tone-index, register-offset}` onsets, exactly the existing figuration schema — extended with an optional register/octave field and a "which voice/role runs it" tag (Bass vs HarmonicFill vs Pad).
- **Meter coupling is the key selector:** waltz/arpeggiated-waltz ⇄ triple meter; oom-pah/stride/walking ⇄ duple; this pairs with the engine's `meter` catalogue. The `SelectTable` ladder gates idiom by (meter, character, arousal): e.g. walking bass under a "groove/jazz" character, Alberti under "classical/ballad," pedal point under "still/low-motion," stride/oom-pah under "march/scherzo."
- Walking bass and pedal point are the two that reach beyond a fixed onset table: walking bass needs a *target-seeking stepwise bass generator* (head toward the next root, fill with passing tones); pedal point needs a *hold-one-pitch-under-changing-harmony* mode. Both are flagged below as needing a small generator function rather than a static row.

---

## Area 4 — Dissonance-as-Intention (RESEARCH-ONLY; build deferred)

Goal: document *what makes dissonance read as intentional/expressive rather than as noise*, so a later slice can map the image's chaos↔order axis onto it. **No mapping is designed here.** The coherence constraints are:

### 4.1 The non-chord-tone taxonomy (each = approach + resolution requirement, predicate-shaped)

A dissonance reads as intentional when it is a *recognized NCT figure* with a satisfied approach and resolution. Each row is `(approached_by, left_by, accent)`:

| NCT | Approached by | Left by (resolution) | Typical accent | Predicate shape |
|---|---|---|---|---|
| Passing tone (PT) | step | step, *same direction* | weak (accented PT exists on strong) | `step_in AND step_out AND same_dir` |
| Neighbor tone (NT) | step | step, *opposite direction* (returns to same pitch) | weak | `step_in AND step_out AND opposite_dir AND start==end` |
| Appoggiatura | **leap** (often up) | step, *opposite* to the leap | **strong** (accented) | `leap_in AND step_out AND opposite_dir AND on_strong_beat` |
| Escape tone (échappée) | step (often up) | **leap**, *opposite* to the step | weak | `step_in AND leap_out AND opposite_dir` |
| Suspension | (prepared as consonance, held) | step **down** to consonance | **strong** | the 3-stage prep/hold/resolve predicate from §1.6 |
| Retardation | (prepared, held) | step **up** to consonance | strong | like suspension but resolves *up* |
| Anticipation | step or leap | **stays** (becomes a chord tone of the next harmony) | weak (on the off-beat before the chord change) | `next_harmony_makes_it_a_chord_tone AND no_motion_into_resolution` |
| Pedal tone | (sustained / held) | resolves when harmony returns to consonance with it | held through; dissonant in the middle | `held_pitch AND harmony_moves_away_then_back` |

The two organizing axes the architect should keep:
- **Approach/resolution by step vs leap** is what distinguishes the figures (passing=step/step, appoggiatura=leap/step, escape=step/leap). The engine's `motion_dir` already provides the per-side direction; "step vs leap" is the |interval| threshold (≤2 vs ≥3).
- **Strong vs weak beat** changes the *character* (an accented appoggiatura/suspension is expressive *because* the dissonance lands on the strong beat and resolves), and changes the *legality* per species (e.g. third-species dissonance is weak-beat-only; suspensions are strong-beat).

### 4.2 How counterpoint controls dissonance (the coherence constraints)

The through-line of common-practice dissonance control — the constraints that keep the dissonant end of the spectrum *coherent* rather than noise:

1. **Every dissonance is prepared and resolved.** A dissonance that is approached and left correctly (per the table) reads as intentional; an unprepared, unresolved dissonance reads as an error/noise. This is the single most important coherence rule.
2. **Dissonance lives on the metrically weaker position unless it is a sanctioned accented figure** (appoggiatura, accented passing tone, suspension). Random strong-beat dissonance reads as noise; *deliberate* strong-beat dissonance that resolves reads as expressive intensity.
3. **Resolution is by step, and (for the accented figures) downward by default.** The ear tolerates a sharp dissonance if it can hear where it is going and the going is small and immediate.
4. **One dissonance at a time per voice; resolve before introducing the next** (except sanctioned chains, e.g. suspension chains, passing-tone runs in a single direction). Stacked unresolved dissonances accumulate into noise.
5. **The consonant frame must remain audible.** Structural/strong-beat verticals stay consonant; dissonance is ornament *over* a consonant skeleton. Lose the skeleton and the dissonance has nothing to be intentional *against*.

### 4.3 The consonant→dissonant spectrum (groundwork only)

A continuum the architect can later attach to an image chaos↔order feature — documented here, **not mapped**:
- **Consonant/ordered end:** only chord tones; dissonance only as unaccented passing/neighbor; strong-beat verticals are perfect/imperfect consonances; predictable resolution. Reads as calm, simple, stable.
- **Middle:** accented NCTs (appoggiaturas, suspensions), richer chords (7ths/9ths — which the engine already gates on saturation), more frequent but still-resolved dissonance. Reads as expressive, yearning, tense-but-coherent.
- **Dissonant/chaotic end:** dense, frequent dissonance, delayed or chained resolutions, weakened tonal frame. What keeps it coherent rather than noise is precisely constraints 1–5: as long as each dissonance is *prepared, resolved, and metrically placed*, the texture stays intentional even when saturated. The coherence floor is "every dissonance is a recognized figure"; remove that and you cross into noise.

The architect's later job (deferred) is to map an image feature onto *how far up this spectrum* to go AND *which figures* to license at a given density — while always keeping constraints 1–5 as the floor.

---

## Contested Areas and Gaps

- **Perfect fourth (ic 5) — consonance or dissonance?** Contested across pedagogies and by context. Between two independent contrapuntal voices a *bare* fourth is treated as a dissonance (it cannot be a stable structural interval in two-voice species writing); inside a fuller chord (as the interval between upper voices over a supporting bass) the fourth is consonant. **Recommendation for the architect:** treat ic 5 as dissonant *only* in the two-voice counterpoint scorer (Area 1); the engine's existing chordal voice-leading (where a fourth between upper voices over a bass is fine) should keep its current behavior. This split is the safe, standard resolution.
- **"Hidden" / "direct" fifths and octaves — how strict?** Strict Fux forbids *all* similar motion into a perfect consonance. Common-practice four-part-harmony pedagogy relaxes this when (a) the upper voice moves by step and (b) it is not the outer voices. **Recommendation:** for the new independent two-voice counter-line, use the strict rule (forbid all similar motion into a perfect) — it is simpler to encode and yields the cleaner two-voice independence the engine lacks. Note it as a tunable.
- **Second-species dissonance: passing tones only, or neighbors too?** Strict Fux: passing tones only on the weak beat. Looser modern pedagogy admits neighbor tones. **Recommendation:** allow passing as HARD-legal, neighbor as PREF-permitted, behind a flag.
- **Cambiata exact shape varies** by source (the "leap of a third" away from the dissonance is constant; whether the figure is 4 or 5 notes and the exact step directions differ slightly between Fux editions and Renaissance practice). Encode one canonical 5-note form and treat variants as out of scope.
- **Gap — affect/character labels for progressions are interpretive.** The affect column in Area 2 is conventional/widely-agreed but not a hard standard; the architect decides which feature ladder selects which progression. Flagged so it isn't treated as a derived fact.
- **Gap — register/voicing of accompaniment idioms.** Sources define the *chord-tone order and subdivision* precisely (encodable) but the *exact register/octave split* (how low the oom-pah bass note, how high the "pah") is performance convention, not a fixed rule. The architect should pick register offsets that fit the engine's existing register bands rather than expecting a canonical number.
- **Out of scope by guardrail:** any specific copyrighted song's actual melody/arrangement. Every skeleton above is a degree-pattern or a figure type (public-domain theory), never a transcription. The blues/axis/doo-wop *skeletons* are theory; a specific song that uses them is not, and is not included.

---

## Encodability notes for the architect

Mapping each area to where it most plausibly lands. The single genuine **new capability** is Area 1's independent counterpoint voice; Areas 2 and 3 are **catalogue-deepening** of existing data tables; Area 4 is groundwork for a later slice.

**Area 1 — Species counterpoint → a new/extended scored predicate in `chord_engine.rs` (THE new capability).**
This is *not* a `mappings.json` change — it is engine logic. It extends the existing `CounterMelody` realizer and reuses what is already there: `interval_class`, `has_parallel_perfects` (already the §1.2 HARD parallel gate), `upper_voice_candidates` (candidate enumeration), `motion_dir`/`MotionDir` (single-line direction), `CONTRARY_BONUS` (generalize to a graded contrary>oblique>similar>parallel score), the `PhrasePosition` cadence markers (drive the §1.3 begin/cadence formulas), and the multi-`NoteEvent`-per-step capability (third/fifth-species density). What's *new*: a `harmonic_class` consonance classifier (§1.1), HARD gates for "structural verticals consonant," "dissonance only as a licensed figure," "approach perfects by contrary/oblique," "no melodic dissonant leaps," and the suspension three-stage predicate (§1.6). Recommend modeling it as fifth-species figure-selection (§1.7): per step, enumerate {sustain, passing, neighbor, suspension, cambiata} candidates → HARD-gate → PREF-score → pick. A few rule thresholds (the relaxable ones in Contested Areas) could be surfaced as `mappings.json` constants, but the rule *logic* is Rust.

**Area 2 — Idiomatic progressions → new rows in `mappings.json` `progression_families` (catalogue-deepening).**
Most entries are one hyphenated roman-numeral string added to warm/cool/neutral, selected by the existing `pick_progression` + `SelectTable` ladders — zero engine change. Three need slightly more: **lament bass** and **Andalusian** are bass-descent-defined (a stepwise descending tetrachord) and **12-bar blues** is a fixed 12-slot form; these are still expressible as roman-numeral rows, but flag that their *identity* is the bass line / the form length, so the architect may want a small "bass-constraint" or "fixed-form" tag on those rows rather than relying on chord symbols alone. Picking among them is an existing `SelectTable` job (gate lament/Andalusian to minor + low valence, blues to a groove feature, axis to high-arousal major).

**Area 3 — Accompaniment idioms → new rows in `mappings.json` `figuration_catalogue` (+ a `texture_catalogue` tag), catalogue-deepening, with two exceptions.**
The fixed-pattern idioms (Alberti already present; broken-chord, arpeggiated waltz, oom-pah, stride, block-comping) are each an onset list in the **existing figuration schema** `{at, tone}` — extended with an optional register-offset and a "which role runs it" field. Selection is meter-coupled via the existing `meter` + character ladders. **Two exceptions need a small generator function (engine, not data):** **walking bass** (a target-seeking stepwise bass that aims at the next chord root and fills with passing tones — it can't be a static onset table because the notes depend on the next chord) and **pedal point** (hold one pitch under changing harmony — already half-present as the `PedalPoint` ostinato; extend it to the accompaniment role). Flag both as logic, the rest as data.

**Area 4 — Dissonance-as-intention → DEFERRED; lands later as predicates in `chord_engine.rs` reusing the Area-1 machinery.**
When built, the NCT taxonomy (§4.1) is the *same* approach/resolution predicate family as Area 1's licensed figures — the suspension predicate, passing/neighbor predicates, etc. are shared. The coherence constraints (§4.2) are the HARD floor. The image chaos↔order → dissonance-density/figure-license mapping is a future `mappings.json` ladder (a `SelectTable` over a chaos feature choosing a density + a permitted-figure set), but that mapping is explicitly **not designed here**. Build it on top of Area 1; do not duplicate the figure predicates.

**Priority recommendation:** build Area 1 first (it is the missing capability and everything else either already exists as data or builds on it), then Area 3's fixed-pattern rows (cheap data wins, immediate textural variety), then Area 2's rows, and defer Area 4 until the counterpoint voice is proven.

---

## Source Summary

Tier 1–2 (authoritative pedagogy / standard references):
- Open Music Theory — *Introduction to Species Counterpoint* and *Embellishing Tones / Nonchord Tones* chapters (standard open pedagogy).
- *Fundamentals, Function, and Form* (Milne/Geneseo open text) — Nonharmonic Tones chapter.
- Ars Nova *Species Counterpoint Manual* — dissonance-handling and fourth-species (suspension) rules.
- Fux *Gradus ad Parnassum* (public-domain primary source) lineage as transmitted by the above.
- musictheory.net lessons — nonharmonic tones (standard reference).

Tier 2–3 (reference encyclopedia for idiom skeletons — degree patterns, public-domain theory):
- Wikipedia entries for *Lament bass*, *Descending tetrachord*, *Andalusian cadence*, *'50s progression*, *Alberti bass*, *Pachelbel's Canon* (used only for the public-domain scale-degree skeleton and the standard affect label, not for any transcription).
- AP Music Theory / Hoffman Academy / StudyBass reference pages — accompaniment-texture definitions (Alberti, walking bass, oom-pah).

All rule formulations cross-checked across at least two sources; the only material disagreements are the four contested items listed in Contested Areas (perfect-fourth status, hidden-fifths strictness, second-species neighbor admission, cambiata exact shape). No claim rests below tier 3, and no copyrighted composition is reproduced — every pattern is a degree-skeleton or figure type.

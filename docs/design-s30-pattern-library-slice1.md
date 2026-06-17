# Design — S30 Pattern-Library Arc, Slice 1 (Species-Counterpoint Voice + Catalogue Deepening)

Date: 2026-06-17
Status: DESIGN ONLY — no implementation in this document. Specifies the build for two file-disjoint lanes.
Grounding: `docs/research-s30-pattern-library.md` (the pattern-theory briefing). Reads only — this design cites the existing engine and does not modify it.

This document delivers two things:

- **(A)** an arc decomposition for the multi-session "musical pattern-library" effort, naming where the deferred ear-gated work docks; and
- **(B)** a complete, test-gated specification for the S30 build slice ("Slice 1") — the largest body of work buildable and validatable **without an audible re-listen**. Slice 1's acceptance is entirely property-test-based. The one ear-gated capability (the chaos↔order → consonance/dissonance "clashing" control) is **explicitly out of Slice 1**; its dock is identified.

---

## 1. ARC DECOMPOSITION

The pattern-library arc promotes the engine from "note-level counter-voice + a small catalogue" to "an independent species-counterpoint voice, a deepened progression/accompaniment catalogue, and (later) an image-driven dissonance spectrum." It decomposes into the following slices. Each slice is independently shippable; later slices build strictly on earlier ones.

### Slice 1 (S30) — Species-Counterpoint Voice + Pure-Data Catalogue Deepening. **TEST-GATED.**
The genuinely new capability: promote the partial counter-voice realizer into a **species-counterpoint-scored independent voice** — per-step figure enumeration {sustain / passing / neighbor / suspension / cambiata}, HARD-gated by first-species + figure-specific invariants, PREF-scored, picked deterministically. Plus pure-data catalogue rows: new idiomatic progressions (research Area 2) and **fixed-pattern** accompaniment idioms (research Area 3). All acceptance is property-test assertions over generated `NoteEvent`s — no listen required. The counter-voice defaults active where a Melody role/section warrants it, so the owner has something to hear in the afternoon, but its **correctness gate is the property net**, not the listen.

This is the slice fully specified in §2–§8 below.

### Slice 2 (S31) — Generator-Backed Accompaniment Idioms. **TEST-GATED, with an optional listen.**
The two Area-3 idioms that cannot be a static onset table because their notes depend on the *next* chord or on *sustaining under changing harmony*: **walking bass** (a target-seeking stepwise bass aiming at the next chord root, filling with passing tones) and **pedal-point accompaniment** (hold one pitch under changing harmony — extends the existing `PedalPoint` ostinato into an accompaniment role). These need small generator functions, not data rows, so they are split out of Slice 1 to keep Slice 1's acceptance purely table-and-predicate shaped. Their bass-line legality (passing tones on weak beats, arrival on chord roots on strong beats) is property-testable; a listen is a confidence check, not the gate.

### Slice 3 (S32) — Bass-Constrained / Form-Defined Progressions. **TEST-GATED.**
The three Area-2 idioms whose *identity is the bass line or the form length*, not a chord-symbol string: **lament bass** (stepwise descending tetrachord 1̂–7̂–6̂–5̂), **Andalusian cadence** (descending tetrachord to a major V), and the **12-bar blues** (a fixed 12-slot form). Slice 1 admits the *chord-symbol approximations* of these as plain progression rows (so the catalogue grows immediately); Slice 3 adds the bass-descent constraint tag and the fixed-form length tag so their defining feature is actually enforced. Deferred from Slice 1 because they need either a new row tag the planner reads or a bass-line post-pass — a wider seam than the pure-row additions Slice 1 keeps to.

### Slice 4 (S33+) — Dissonance-as-Intention: the chaos↔order "clashing" control. **EAR-GATED. THE DEFERRED SLICE.**
Research Area 4. Maps an image chaos↔order feature onto *how far up the consonant→dissonant spectrum to go* and *which non-chord-tone figures to license at a given density*, always keeping the §4.2 coherence constraints (every dissonance prepared+resolved, weak-beat-default, step-resolution, one-at-a-time, audible consonant frame) as the HARD floor.

**Where it docks:** directly on top of Slice 1's figure-selection machinery. Slice 1 builds the *figure predicates* (passing / neighbor / suspension / cambiata, each as an approach+resolution predicate) and the per-step enumerate→gate→score→pick loop. Slice 4 adds (a) the remaining NCT figures (appoggiatura, escape tone, retardation, anticipation) as the *same* predicate family, and (b) a **new `SelectTable` ladder in `mappings.json`** over a chaos/order image knob that chooses a *dissonance-density target* and a *permitted-figure set*, which the figure-selection loop then reads as a license mask. Because Slice 1 already routes every dissonance through a licensed-figure gate, Slice 4 is a *widening of the license set + a data ladder*, not a rewrite. **Slice 4 is ear-gated because the affect mapping (which density reads as "expressive tension" vs "noise") is a musical-judgment call the property net cannot adjudicate** — the net can prove every dissonance is *legal*, but only a listen confirms it is *expressive*. This is why it is excluded from a no-listen slice.

> Sequencing note: Slices 2 and 3 are independent of each other and of Slice 4; either may run before the other after Slice 1 lands. Slice 4 strictly requires Slice 1.

---

## 2. CURRENT-STATE ANALYSIS (line refs)

All references are `src/chord_engine.rs` unless noted. The counter-voice realizer and its helpers already exist as a partial implementation; Slice 1 deepens them in place.

### 2.1 The existing counter-voice realize arm
- **`OrchestralRole::CounterMelody` arm of `realize_step`** — lines **1703–1792**. This is the entire per-step counter-voice realization: it recomputes the melody pitch this/prev step (1718–1720), detects held-chord / melody-static periods (1722–1732), computes the advancing held-run target (1751), seeds the prior counter pitch (1752), calls `pick_counter_pitch` (1753–1761), and emits rhythm (1765–1791). It is reached **only** when an instrument is assigned the `CounterMelody` role, which never happens under the identity profile (the freeze path) — the header comment at 1709–1710 states this explicitly.

### 2.2 The scoring core to be deepened
- **`pick_counter_pitch`** — lines **2957–3080**. The current scorer: enumerates chord-tone candidates (`counter_candidate_pitches`, 2978), HARD-rejects parallel perfect 5ths/8ves vs the melody via `has_parallel_perfects` over the 2-voice `[melody, counter]` pair at T→T+1 (3004–3013), and PREF-scores motion size minus `CONTRARY_BONUS` for contrary/oblique motion (3015–3035), plus the unison-double penalty (3037–3040), held-period move penalty (3042–3048), and `ROOT_PC_BIAS` tie-break (3050–3056). **This is exactly the enumerate→HARD-gate→PREF-score→pick shape Slice 1 deepens into species scoring.** Today it only handles the *sustain* figure (one chord tone per step); Slice 1 adds the dissonant figures.

### 2.3 The reusable helpers (build ON these — do not reinvent)
- **`interval_class(a, b)`** — lines **2650–2652**. `|a−b| mod 12`. The foundation for the new `harmonic_class` classifier.
- **`has_parallel_perfects(a, b)`** — lines **2661–2674**. HARD parallel-perfect gate over voice pairs at T and T+1, with the "both voices move" (non-oblique) condition. Already the §1.2 parallel-perfects rule. Reused as-is; a new call site (not an edit) adds the species checks.
- **`upper_voice_candidates(pc, from, max_motion)`** — lines **2627–2645**. Octave-seating candidate enumeration within a motion cap. Reused by `counter_candidate_pitches` (2926).
- **`motion_dir(prev, now)` / `enum MotionDir`** — lines **2730–2744**. Single-line direction (Up/Down/Hold). The per-side direction the figure predicates need (step-in/step-out same-vs-opposite direction).
- **`CONTRARY_BONUS`** — line **2699** (value 24). The single contrary/oblique bonus; Slice 1 generalizes it to a *graded* contrary > oblique > similar > parallel score.
- **`COUNTER_CEILING` / `FILL_REGISTER_FLOOR`** — lines **2690 / 1190**. The counter register band [55, 67).
- **`counter_candidate_pitches` / `nearest_counter_tone`** — lines **2918–2945 / 2897–2901**. Chord-tone candidate generation in the counter band.
- **`PhrasePosition`** — lines **507–517** (`PhraseStart` / `Interior` / `HalfCadence` / `PerfectAuthenticCadence`). Drives the §1.3 begin/cadence formulas. Carried on every `StepPlan` (line 539) and read in the arm via `step.position`.
- **Multi-`NoteEvent`-per-step capability** — the arm already returns a `Vec<NoteEvent>` with independent `offset_ms` (e.g. 1770–1774); third-species density (two-to-four notes per step) is therefore already expressible without any new mechanism.

### 2.4 The data seam (catalogue deepening lands here)
- **`progression_families`** — `assets/mappings.json` lines **42–46** (`warm` / `cool` / `neutral` arrays of hyphenated roman-numeral strings). `ChordEngine::pick_progression` (`src/chord_engine.rs` 119–145) splits a chosen row on `-`; `generate_chords` (170+) realizes each symbol via `roman_to_chord_complex` (302). **A new progression is one new string in an array — zero Rust change.**
- **`figuration_catalogue`** — `assets/mappings.json` lines **222–231** (the `{at, tone}` onset schema; Alberti is the worked example). `FigurationSpec` / `FigurationOnset` are defined in `src/composition.rs` **531–557**; the mapper `figured_bed` (`src/chord_engine.rs` 1880–1932) reads `onset.at` (offset within step) and `onset.tone` (seated inner-voice index, cycled modulo the seated count) and `onset.hold_frac`. **A new fixed-pattern idiom is one new catalogue row — zero Rust change**, provided its onsets express in the existing `{at, tone, hold_frac}` schema over the already-seated inner tones.
- **`texture_catalogue` / `texture` SelectTable** — `assets/mappings.json` **216–242**. `pad_bed_counter` (line 219) is the profile that seats a `CounterMelody` layer; the `texture` SelectTable rule at **238–240** activates it when `foreground_energy ≥ 0.35 AND fg_bg_contrast ≥ 0.20`. **The counter-voice is therefore already deterministically activated by image features** — Slice 1 does not need to add an activation path, only to deepen what the activated voice does.

---

## 3. PROPOSED CHANGES PER FILE (complete signatures, no bodies)

Two file-disjoint lanes. The **music-theory lane** is single-writer of `src/chord_engine.rs`. The **implementer lane** owns `src/composition.rs` + `assets/mappings.json` (+ any new non-craft file). The seam between them is precisely §3.3.

### 3.1 Music-theory lane — `src/chord_engine.rs` (counterpoint craft)

All additions are **private** functions/types and **new constants**, reached only from the existing `CounterMelody` realize arm (lines 1703–1792). Nothing here is `pub`; nothing changes `realize_step`'s public 7-param signature; nothing is reachable on the identity/freeze path.

```rust
// --- §1.1 consonance/dissonance classifier (the new foundation predicate) ---

/// Common-practice harmonic class of the vertical interval between two MIDI notes,
/// via `interval_class`. ic 0/7 = perfect consonance; 3/4/8/9 = imperfect consonance;
/// 1/2/6/10/11 = dissonance; ic 5 (perfect fourth) classified per `FOURTH_IS_DISSONANT`
/// (contested-decision #1; dissonant in the two-voice counter scorer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HarmonicClass {
    PerfectConsonance,
    ImperfectConsonance,
    Dissonance,
}

fn harmonic_class(a: u8, b: u8) -> HarmonicClass;

/// True iff the counter scorer treats the bare perfect fourth (ic 5) as a dissonance.
/// Contested-decision #1: `true` for the independent two-voice counter-line (the safe,
/// standard resolution). A `mappings.json` constant could later surface this; Slice 1
/// pins it in Rust as the documented default.
const FOURTH_IS_DISSONANT: bool = true;

// --- §1.x relative (pairwise) motion of two voices ---

/// The four relative-motion types of two voices, in preference order
/// (Contrary best, Parallel worst). Distinct from single-line `MotionDir`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelMotion {
    Contrary,
    Oblique,
    Similar,
    Parallel,
}

/// Pairwise relative motion from voice-pair (a_prev,b_prev) to (a_now,b_now).
/// Parallel == Similar AND the harmonic interval class is preserved.
fn rel_motion(a_prev: u8, a_now: u8, b_prev: u8, b_now: u8) -> RelMotion;

/// Graded score term (lower-is-better convention of `pick_counter_pitch`) for a
/// relative-motion type. Generalizes the single `CONTRARY_BONUS` into the full
/// contrary > oblique > similar > parallel gradient (§1.2 PREF). Sized so the
/// gradient orders motion types without overriding any HARD reject.
fn rel_motion_score(m: RelMotion) -> i32;

// --- §1.2 / §1.3 HARD gates over a voice-pair transition ---

/// HARD (§1.2): approaching a PERFECT consonance must be by Contrary or Oblique
/// motion (forbids direct/hidden fifths & octaves). Contested-decision #2: strict
/// form (forbid ALL similar motion into a perfect), gated by `HIDDEN_PERFECTS_STRICT`.
const HIDDEN_PERFECTS_STRICT: bool = true;

fn approach_perfect_is_legal(
    m_prev: u8, m_now: u8,
    c_prev: u8, c_now: u8,
) -> bool;

/// HARD (§1.2 melodic): the counter line must not LEAP by a dissonant melodic
/// interval (a melodic 7th, or any augmented interval such as the tritone).
fn melodic_leap_is_legal(prev: u8, now: u8) -> bool;

// --- §1.4–§1.6 the licensed dissonant figures (each = approach+resolution predicate) ---

/// The per-step figure the counter sounds. `Sustain` is the first-species default
/// (one consonant chord tone). The rest are the licensed dissonances; each carries
/// the pitches needed to emit it and is only producible when its predicate holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CounterFigure {
    Sustain,      // one consonant chord tone (today's behavior)
    Passing,      // §1.4/1.5: dissonance approached & left by step, same direction
    Neighbor,     // §1.5: step away & step back (PREF, behind NEIGHBOR_ALLOWED)
    Suspension,   // §1.6: prepared-consonant, held, dissonant-on-strong, resolves DOWN by step
    Cambiata,     // §1.5: the canonical 5-note changing-note figure (one sanctioned leap-from-dissonance)
}

/// Contested-decision #3 (second-species dissonance): passing is HARD-legal,
/// neighbor is PREF-permitted behind this flag (`true` admits neighbors).
const NEIGHBOR_ALLOWED: bool = true;

/// §1.4 passing-tone predicate: candidate dissonant note legal iff step-in AND
/// step-out AND same direction, both ends consonant.
fn is_legal_passing(prev: u8, cand: u8, next_resolution: u8, cf_at_cand: u8) -> bool;

/// §1.5 neighbor predicate: step-away then step-back to the start pitch, both ends
/// consonant, opposite directions.
fn is_legal_neighbor(prev: u8, cand: u8, return_to: u8, cf_at_cand: u8) -> bool;

/// §1.6 suspension three-stage predicate over three consecutive counter states and
/// the bass/CF: prep consonant -> held same pitch -> now dissonant on strong ->
/// resolves DOWN by step to a consonance. Uses the (dissonance->resolution) table
/// {7-6, 4-3, 9-8 upper; 2-3 bass}.
fn is_legal_suspension(
    prep: u8, held: u8, resolution: u8,
    cf_prep: u8, cf_held: u8, cf_resolution: u8,
) -> bool;

/// §1.5 cambiata: recognize the one canonical 5-note form (consonance -> step down
/// to dissonance -> leap down a third -> step up -> step up). Permits the otherwise-
/// illegal leap-away-from-dissonance when the whole template matches.
fn is_legal_cambiata(figure: &[u8], cf: &[u8]) -> bool;

// --- §1.3 begin / cadence boundary formulas ---

/// HARD (§1.3): the counter's FIRST vertical (PhraseStart) must be a perfect
/// consonance (unison/5th/octave above; unison/octave below). Returns the legal
/// opening candidate set filtered to perfect-consonant verticals.
fn opening_candidates(chord: &Chord, cf_first: u8, counter_above: bool) -> Vec<u8>;

/// HARD (§1.3): at a PerfectAuthenticCadence step, resolve the counter by stepwise
/// CONTRARY motion onto the octave/unison with the CF (the clausula), with the
/// penultimate vertical a major 6th (counter above) or minor 3rd (below).
fn cadence_resolution_pitch(
    chord: &Chord,
    cf_now: u8,
    counter_prev: u8,
    cf_prev: u8,
) -> u8;

// --- §1.7 the fifth-species figure-selection driver (the new top of pick_counter_pitch) ---

/// Enumerate the legal {Sustain, Passing, Neighbor, Suspension, Cambiata} candidates
/// for this step under the HARD gates (only-consonant structural verticals; dissonance
/// only as a licensed figure with correct approach+resolution; approach perfects by
/// contrary/oblique; no melodic dissonant leaps; begin/cadence formulas), then PREF-
/// score survivors (graded relative motion, imperfect>perfect interior, stepwise>leaping
/// melody, leap recovery, capped parallel-imperfect runs, suspension-chain & cambiata
/// rewards) and pick the best. This REPLACES the body of the current sustain-only scorer
/// while preserving the as-built sustain pick byte-for-byte when only Sustain is legal
/// (the default/equivalence path — see §5). Private; signature additions are private
/// params threaded from the existing arm's already-derived locals.
fn pick_counter_figure(
    chord: &Chord,
    prev_counter: u8,
    m_prev: Option<u8>,
    m_now: Option<u8>,
    mel_dir: MotionDir,
    position: PhrasePosition,        // begin/cadence formulas (read from step.position)
    next_chord: Option<&Chord>,      // resolution target for passing/suspension look-ahead
    force_move: bool,                // unchanged held-period semantics
    held_target: Option<u8>,         // unchanged held-run rotation semantics
) -> (u8, CounterFigure);

// --- new PREF constants (siblings of CONTRARY_BONUS, all private) ---
const IMPERFECT_PREF: i32;            // §1.2 prefer 3rds/6ths over perfects interior
const PARALLEL_IMPERFECT_RUN_CAP: usize; // §1.2 cap consecutive parallel 3rds/6ths
const LEAP_RECOVERY_BONUS: i32;       // §1.2 step-back-after-leap reward
const SUSPENSION_CHAIN_BONUS: i32;    // §1.6 reward consecutive legal suspensions
```

**Note on `pick_counter_pitch`:** Slice 1 either renames its entry point to `pick_counter_figure` (returning the figure tag alongside the pitch) or keeps `pick_counter_pitch` as a thin wrapper that calls the new driver and discards the tag, so the realize arm's call site (1753) changes minimally. The single-writer of this file picks; the requirement is only that the **sustain-only path is byte-preserved** (§5).

### 3.2 Implementer lane — `src/composition.rs` + `assets/mappings.json`

**`src/composition.rs`** — Slice 1 adds **no new Rust types for the data rows** if the new figuration idioms fit the existing `FigurationSpec`/`FigurationOnset` schema (they do — see §4.2). The one *optional* implementer-side addition, only if a register split is wanted for oom-pah/stride (it is NOT required for Slice 1 — see §7):

```rust
// OPTIONAL, behind a serde default so the JSON byte-shape and all existing rows
// are unchanged. Adds a per-onset register offset (octaves) so an idiom can place
// its "pah"/stab above its bass note. `#[serde(default)]` = 0 => byte-identical to
// every existing row, which omit it. If omitted from Slice 1, oom-pah/stride are
// deferred to a slice that adds it; the simpler fixed idioms (broken-chord, block-
// comping, arpeggiated waltz) need NO new field and ARE in Slice 1.
pub struct FigurationOnset {
    pub at: f32,
    pub tone: u8,
    #[serde(default = "one_f32")]
    pub hold_frac: f32,
    #[serde(default)]            // 0 => no register shift => byte-identical default
    pub register_octaves: i8,    // NEW (optional); read by figured_bed
}
```

> If `register_octaves` is added, the mapper read is a one-line addition in `figured_bed` (chord_engine.rs) — that is a **music-theory-lane edit** because `figured_bed` lives in `chord_engine.rs`. To keep the lanes clean, **Slice 1's recommendation is to NOT add `register_octaves`** and defer oom-pah/stride to Slice 2 (alongside walking bass), so the implementer's Slice-1 work is **pure `mappings.json` data** plus, at most, the serde-default field if the lead wants it. See §7 contested-decision wrap-up and §8.

**`assets/mappings.json`** — the implementer's Slice-1 deliverable is data only (§4): new `progression_families` rows (Area 2 pure-row idioms) and new `figuration_catalogue` rows (Area 3 fixed-pattern idioms), plus the `SelectTable` gating that selects them.

### 3.3 The seam between the lanes (precise)
- The **music-theory lane** owns *what a counter note is* — the figure enumeration, the HARD gates, the PREF scoring, the begin/cadence formulas. All of it is private code inside `chord_engine.rs`, reached from the existing `CounterMelody` arm. It reads `step.position` (already on `StepPlan`), the chord (already passed), and the melody pitches (already recomputed in the arm). **It requires no new field from `composition.rs` and no new data from `mappings.json`** — every input it needs already exists on the borrowed `StepContext`/`StepPlan`. This is the key to the byte-freeze (§5).
- The **implementer lane** owns *which progression and which accompaniment figure a section uses* — the `mappings.json` rows and the `SelectTable` condition-ladders that select them deterministically from image features. It does **not** touch counterpoint logic.
- **They are file-disjoint:** music-theory writes only `chord_engine.rs`; implementer writes only `composition.rs` + `mappings.json`. The counter-voice's *selection* (does this section even carry a CounterMelody layer?) is already an implementer-domain `texture` SelectTable decision (the `pad_bed_counter` row); the counter-voice's *behavior* is music-theory-domain. The seam is therefore the existing `OrchestrationProfile.layers` containing `CounterMelody` — set by data, consumed by craft. No new shared field crosses it in Slice 1.

---

## 4. mappings.json ROW SCHEMAS + EXAMPLE ROWS

### 4.1 Area 2 — new `progression_families` rows (pure roman-numeral strings)
Schema (unchanged): a hyphen-joined roman-numeral string in `warm` / `cool` / `neutral`. `pick_progression` splits on `-`; each symbol is realized by the existing `roman_to_chord_complex`. **Only symbols the existing roman parser already accepts** are admitted in Slice 1 (the parser supports the diatonic numerals and the borrowed `bVII`/`bVI`/`bIII` already present in the `cool` rows). Symbols requiring new parsing (e.g. a literal `V7` quality on a I chord for blues) are **deferred to Slice 3**.

Concrete additions (each a new array entry; existing rows unchanged):

```jsonc
"progression_families": {
  "warm": [
    "I-vi-IV-V", "ii-V-I", "I-V-vi-IV",
    "vi-IV-I-V",                       // axis rotation (high-arousal major)
    "IV-I-V-vi",                       // axis rotation
    "I-IV-vii-iii-vi-ii-V-I",          // circle-of-fifths sequence (propulsive)
    "I-vi-IV-ii"                       // descending-thirds sequence (gentle)
  ],
  "cool": [
    "i-bVII-bVI-V", "i-iv-V", "i-bVI-bIII-bVII",
    "i-bVII-bVI-V",                    // Andalusian chord-symbol approximation (Slice 1);
                                       //   bass-tetrachord ENFORCEMENT is Slice 3
    "i-VII-VI-V",                      // lament-bass chord-symbol approximation (Slice 1);
                                       //   stepwise-descending-bass tag is Slice 3
    "iv-V"                             // Phrygian half-cadence tag (pairs with Half boundary)
  ],
  "neutral": [
    "I-V-vi-iii-IV-I-IV-V",
    "I-vi-IV-V"                        // 50s / doo-wop (nostalgic, common-tone-rich)
  ]
}
```

> The lament/Andalusian rows are admitted in Slice 1 **as their chord-symbol approximation only**, and labeled in the commit/comment as such, because their *defining bass tetrachord* is a Slice-3 tag. This grows the catalogue now without claiming the constraint is enforced. Affect-gating (which `SelectTable` rule selects each) is an implementer decision per the research's Area-2 guidance (lament/Andalusian → minor + low valence; axis → high-arousal major; doo-wop → mid-valence major). These gates ride the **existing** `key_scheme`/`character` ladders or a new progression-selection ladder if the implementer adds one; no new knob is required (all referenced knobs — arousal, valence, complexity — already exist).

### 4.2 Area 3 — new `figuration_catalogue` rows (fixed-pattern idioms, existing `{at, tone, hold_frac}` schema)
Schema (unchanged): `{ "id", "voices", "onsets": [ { "at", "tone", "hold_frac"? } ] }`. `at` = fraction of step (0..1); `tone` = seated inner-voice index, cycled modulo the seated count; `hold_frac` = fraction of the gap to the next onset. The mapper (`figured_bed`) reads exactly these. **In-Slice-1 idioms are those expressible with no new field** (no register split, no next-chord dependency):

```jsonc
"figuration_catalogue": [
  { "id": "block",   "onsets": [] },
  { "id": "alberti", "voices": 3,
    "onsets": [ {"at":0.0,"tone":0}, {"at":0.25,"tone":2},
                {"at":0.5,"tone":1}, {"at":0.75,"tone":2} ] },

  // NEW — broken-chord / arpeggiation (ascending): root->3rd->5th, one per quarter.
  { "id": "broken_chord_up", "voices": 3,
    "onsets": [ {"at":0.0,"tone":0}, {"at":0.25,"tone":1},
                {"at":0.5,"tone":2}, {"at":0.75,"tone":1} ] },

  // NEW — broken-chord (rolling/wave): non-monotone, no obvious top accent.
  { "id": "broken_chord_wave", "voices": 3,
    "onsets": [ {"at":0.0,"tone":0}, {"at":0.25,"tone":2},
                {"at":0.5,"tone":1}, {"at":0.75,"tone":2} ] },

  // NEW — arpeggiated waltz (triple feel within the step): bass tone then two uppers,
  // longer first onset (the downbeat lilt). Pairs with a triple meter when the meter
  // catalogue grows; harmless in 4/4 as a 3-onset arpeggiation.
  { "id": "arp_waltz", "voices": 3,
    "onsets": [ {"at":0.0,"tone":0,"hold_frac":1.0}, {"at":0.33,"tone":1},
                {"at":0.66,"tone":2} ] },

  // NEW — block-chord comping on the off-beats (beats 2 & 4 within the step): all seated
  // tones together, struck on the weak positions. tone cycles the whole seated bed.
  { "id": "block_comp_24", "voices": 3,
    "onsets": [ {"at":0.25,"tone":0,"hold_frac":0.5},
                {"at":0.75,"tone":0,"hold_frac":0.5} ] }
]
```

> **Deferred (need a new field or a generator):** *oom-pah* and *stride* need a register split (bass note low vs "pah"/stab higher) — they require the optional `register_octaves` field and are **deferred to Slice 2** with walking bass and pedal point, keeping Slice 1's data field-clean. *Walking bass* and *pedal-point accompaniment* are generator-backed (Slice 2). The four rows above are the complete, no-new-field Slice-1 set.

Each new figuration row also needs a `texture_catalogue` profile that references it (mirroring `pad_figured` which references `alberti`) and a `texture` SelectTable rule that selects that profile from meter/character/arousal — pure implementer data, gated per the research's meter-coupling guidance (broken-chord under lyrical/ballad; block-comp under groove; arp-waltz under triple-feel). All referenced knobs already exist.

---

## 5. BYTE-FREEZE ARGUMENT

**Claim: Slice 1 leaves `src/engine.rs` byte-unchanged (sha256 = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`, confirmed unmoved at design time) and every behavioral freeze witness survives.**

### 5.1 Why `engine.rs` does not move
`engine.rs` carries the byte-frozen shared core and the public `realize_step` 7-param signature. Slice 1's entire new capability lives in `chord_engine.rs` private code reached from the **existing** `CounterMelody` realize arm. That arm already derives, from the borrowed `StepContext`/`StepPlan`, *every input* the species machinery needs:
- the chord and `step.position` (PhrasePosition) — already on `StepPlan` (line 539);
- the melody pitch this/prev step — already recomputed in the arm (1718–1720);
- the prior/next `StepPlan` — already reachable via `ctx.section.steps` (1713);
- held-run / force-move state — already computed (1722–1751).

The one look-ahead the new figure predicates want — the **next chord** (for passing/suspension resolution targets) — is reachable the same way the *prior* step is: `ctx.section.steps.get(ctx.step_in_section + 1)`, read inside `chord_engine.rs` off the **already-borrowed** `ctx`. **No new field is threaded through `engine.rs`, `StepContext`, or `Section`.** This is the proven S29 discipline (ride existing ctx/Section/role fields). Therefore `engine.rs` stays byte-identical and `realize_step`'s public signature is untouched.

> **Freeze-sensitive flag (explicit, per the hard constraint):** the *only* thing that could force an `engine.rs` touch is if the species scorer needed a datum that is NOT re-derivable from the borrowed context. The audit above shows it is not — every input is already present or reachable via `ctx.section.steps`. **I am asserting `engine.rs` does NOT change, with the next-chord look-ahead as the one new read, satisfied entirely within `chord_engine.rs` off the existing borrow.** Fallback if a future need arises (it does not in Slice 1): a `#[serde(skip)]` planner-filled field on `Section` (the blessed `figuration_resolved` route), which still leaves `engine.rs` and the JSON byte-shape unchanged — but Slice 1 needs neither.

### 5.2 Why each behavioral witness survives
- **`engine_equivalence` 9/9 BYTE-GREEN** (`tests/engine_equivalence.rs`): this net exercises the **identity profile** — no `CounterMelody` instrument is assigned (the identity `OrchestrationProfile` has empty `layers`). The `CounterMelody` arm (1703) is therefore **never reached** on any equivalence step. All Slice-1 code is downstream of that arm. The equivalence batch is byte-identical because no equivalence step ever executes a single line of new code. The realize-step public path for Bass/Melody/HarmonicFill/Pad is untouched.
- **Goldens 240ms / 114 / 84 / 36 / 79 UNMOVED** (`tests/engine_equivalence.rs` P5/P6 pins and siblings): these pin role-derived bass/melody pitch and the cadence/velocity goldens on the **identity / free-select path**, which Slice 1 does not modify (no edit to `role_pitch`, `realize_velocity`, `realize_rhythm`, or any non-CounterMelody arm). The cadence golden specifically rides `base_frac`/the cadence early-return, which the CounterMelody arm does not touch. Unmoved.
- **The identity / home_only / single_section_default path inserts NOTHING new:** these paths produce no `CounterMelody` layer, so the arm never fires and emits no events. Slice 1 adds zero events to any non-counter instrument.
- **Sustain-only byte-preservation (the counter's OWN default):** even where the `CounterMelody` arm *is* active, the new `pick_counter_figure` driver MUST return the **exact same pitch** the current `pick_counter_pitch` returns whenever only the `Sustain` figure is legal — i.e. on every step where no licensed dissonance applies, the species scorer reduces to today's chord-tone scorer. This is a hard requirement on the music-theory lane and is itself property-tested (see §6, test PT-0). It guarantees that the existing counter-voice tests (`tests/saliency_s18.rs`) that pin the as-built sustain behavior do not move except where a dissonant figure is *intentionally* now produced (and those new cases are pinned by the new property tests, not the old goldens).
- **`realize_step` PUBLIC 7-param signature FROZEN:** Slice 1 adds only *private* functions, *private* params (threaded between private fns), and *private* constants in `chord_engine.rs`. The public signature is unchanged.

---

## 6. PROPERTY-TEST ACCEPTANCE LIST

All acceptance is deterministic assertions over generated `NoteEvent`s / pitch sequences from a hand-built `CounterMelody`-bearing section (the existing test pattern at `chord_engine.rs` 5308–5390 builds exactly such a section by hand, headless, RNG-free — the new tests extend it). **No re-listen.** The Test Engineer writes these as a new `tests/counterpoint_s30.rs` plus pins in the existing nets.

- **PT-0 — Sustain byte-preservation (freeze):** for every step where no dissonant figure is licensed, `pick_counter_figure` returns the *identical* pitch the as-built `pick_counter_pitch` returns. Assert pitch-equality against the recorded as-built sequence over a fixed battery of sections. (This is the §5.2 byte-preservation guard.)
- **PT-1 — Voice independence (no parallel perfect 5ths/8ves):** over the generated `[melody, counter]` pitch pair, assert `has_parallel_perfects` is FALSE at **every** T→T+1 transition (checked at T and T+1, the existing predicate's contract). No parallel unisons/5ths/octaves between the counter and the line it counters, ever.
- **PT-2 — Approach-to-perfect legality (no direct/hidden perfects):** for every transition whose *resulting* vertical is a perfect consonance, assert the relative motion was `Contrary` or `Oblique` (never `Similar`/`Parallel`) — the strict `HIDDEN_PERFECTS_STRICT` rule.
- **PT-3 — Motion-preference distribution:** over a representative section battery, assert the empirical distribution of relative-motion types favors contrary+oblique over similar+parallel (e.g. contrary+oblique strictly the majority). Deterministic because the input images/sections are fixed; assert exact counts against a recorded baseline, not a fuzzy ratio.
- **PT-4 — Dissonance only on permitted positions, with correct resolution:** for every counter note classified `Dissonance` against the CF, assert it is a *recognized licensed figure* — passing (step-in/step-out/same-dir, both ends consonant), neighbor (step-away/step-back/opposite-dir, start==end), suspension (prep-consonant→held→dissonant-on-strong→resolves DOWN by step to a consonance), or cambiata (the 5-note template). Any dissonance failing every figure predicate fails the test. **No unprepared/unresolved dissonance.**
- **PT-5 — Suspension shape:** every `Suspension` figure satisfies the three-stage predicate and resolves by a single downward step to a consonance, drawn from the {7-6, 4-3, 9-8, 2-3} table. Assert the held pitch equals the preparation pitch and the resolution is exactly −1 or −2 semitones to a consonant vertical.
- **PT-6 — Leap recovery:** after any melodic leap ≥ a fourth in the counter, assert the next motion is a step in the opposite direction (or, after two same-direction leaps, a step back). No melodic dissonant leap (7th/augmented) appears anywhere in the counter line (`melodic_leap_is_legal` holds at every transition).
- **PT-7 — No unison collapse:** assert the counter never doubles the melody's exact pitch (`counter != melody` at every simultaneous sounding) — the existing `COUNTER_UNISON_PENALTY` made dominant; now a hard assertion.
- **PT-8 — Begin/cadence formulas:** at a `PhraseStart` step the counter's vertical with the CF is a perfect consonance; at a `PerfectAuthenticCadence` step the counter resolves by stepwise contrary motion onto the octave/unison, with the penultimate vertical a major 6th (above) or minor 3rd (below).
- **PT-9 — Determinism:** running the same hand-built section twice yields identical `NoteEvent` sequences (no `thread_rng` reached in the figure/voice selection); assert byte-equality across two runs.
- **PT-10 — Data-row validity (implementer lane):** the new `progression_families` rows each parse via the existing roman parser into well-formed chords (every symbol resolves; adjacent chords share the voice-leading-required pitch class where the existing net demands it); the new `figuration_catalogue` rows each have 2..=4 ascending-`at` onsets in [0,1) with valid `tone`/`hold_frac`, and `figured_bed` emits the expected onset count/positions for each. (Extends `tests/figuration_s20.rs` and the `generate_chords` net.)
- **PT-FREEZE — the existing witnesses, re-run unchanged:** `engine_equivalence` 9/9 byte-green; goldens 240/114/84/36/79 unmoved; the engine.rs sha256 byte-anchor (the `affect_s22.rs`-style witness) still equals the frozen value. These are the §5 guards, asserted in CI as-is.

---

## 7. RISKS / TRADE-OFFS + RESOLUTION OF THE THREE CONTESTED DECISIONS

### Contested decision #1 — perfect fourth (ic 5) consonant or dissonant?
**RESOLVED: dissonant, but ONLY in the two-voice counter scorer.** Pin `FOURTH_IS_DISSONANT = true` inside the counter-voice `harmonic_class` (so a bare 4th between the independent two voices is treated as a dissonance requiring a licensed figure). The engine's *existing chordal voice-leading* (`voice_lead_one`, where a 4th between upper voices over a supporting bass is fine) keeps its current behavior — it does not call `harmonic_class` and is untouched. This is the research's safe, standard split. **Risk:** if a future slice wants the 4th classification tunable, surface the constant in `mappings.json`; Slice 1 does not, to avoid a JSON-shape change.

### Contested decision #2 — hidden/direct fifths strictness?
**RESOLVED: strict (forbid ALL similar motion into a perfect consonance) for the new two-voice counter-line.** Pin `HIDDEN_PERFECTS_STRICT = true`. The strict rule is simpler to encode and yields the cleaner two-voice independence the engine currently lacks; the relaxation (allow hidden perfects when the upper voice steps) belongs to four-part chordal pedagogy, not the bare two-voice line. Noted as a tunable constant. **Trade-off:** strictness occasionally forces an oblique/contrary pick where a stepwise-similar approach would also have been acceptable in looser practice — musically conservative, never wrong.

### Contested decision #3 — which idioms need a generator fn vs a static data row?
**RESOLVED by the slice split:**
- **Static data rows (Slice 1):** broken-chord (up + wave), arpeggiated waltz, block-chord comping — all express in the existing `{at, tone, hold_frac}` schema over already-seated tones. Plus all Area-2 pure roman-numeral rows.
- **Need a new optional field, deferred (Slice 2):** oom-pah, stride — need a register split (low bass vs higher stab), i.e. the optional `register_octaves` onset field. Deferred so Slice 1's implementer work stays pure-data with no schema field and no `figured_bed` edit (which would cross into the music-theory file).
- **Need a generator fn, deferred (Slice 2):** **walking bass** (target-seeking stepwise bass toward the next chord root, passing tones between — cannot be a static onset table because the notes depend on the *next* chord) and **pedal-point accompaniment** (hold one pitch under changing harmony — extends the existing `PedalPoint` ostinato). Both are logic, not data; they belong with oom-pah/stride in the generator slice.
- **Bass-or-form-constrained progressions, deferred (Slice 3):** lament-bass and Andalusian are admitted in Slice 1 *as chord-symbol approximations only* (a row each), with the stepwise-descending-bass-tetrachord enforcement deferred; the 12-bar blues is a fixed-12-slot *form*, deferred entirely (it also wants dominant-7th quality on I/IV, a roman-parser extension).

### Other risks
- **R-A (figure interaction with held-run rotation):** the existing arm has a held-run rotation path (`advancing_seed_counter`) that already advances the counter through chord tones under static harmony. The new dissonant figures must not fight it: in Slice 1, dissonant figures are only *enabled on changing-chord / moving-melody steps*; the held-run rotation path stays sustain-only. This keeps the held-period behavior byte-stable (PT-0 covers it) and avoids two mechanisms picking different pitches for the same step.
- **R-B (next-chord look-ahead at section end):** the resolution target for a passing/suspension figure on the last step of a section has no next chord. Resolve to the *current* chord's nearest consonance (the existing fallback shape), and forbid initiating a figure that cannot resolve — a HARD gate, tested by PT-4.
- **R-C (catalogue selection collisions):** adding progression rows changes which row a `SelectTable` ladder picks only if a *new selecting rule* is added; the new rows are inert until a ladder selects them. Recommend the implementer add explicit selection rules (so the new idioms actually fire) but keep the *default* rows unchanged so existing image→progression behavior is preserved unless a feature crosses a new threshold. PT-10 + the existing nets guard this.

---

## 8. MIGRATION PATH (file-disjoint, ordered)

The two lanes are file-disjoint and can largely proceed in parallel; the ordering below sequences the dependency (the property net needs both lanes' surfaces).

1. **Music-theory lane (chord_engine.rs), step 1 — classifier + relative motion + gates.** Add `HarmonicClass`/`harmonic_class`, `RelMotion`/`rel_motion`/`rel_motion_score`, `approach_perfect_is_legal`, `melodic_leap_is_legal`, and the new constants. No behavior change yet (unused). Compiles green; equivalence still byte-green (nothing reached on identity).
2. **Music-theory lane, step 2 — figure predicates + driver.** Add `CounterFigure`, the four figure predicates, the begin/cadence formula fns, and `pick_counter_figure`. Wire the existing `CounterMelody` arm (1753) to call it. **Enforce PT-0 (sustain byte-preservation) first** — the driver must reduce to today's pitch on sustain-only steps before any dissonant figure is enabled. Then enable figures on changing-chord/moving-melody steps. Reads next-chord via `ctx.section.steps.get(si+1)` — no engine.rs touch.
3. **Implementer lane (mappings.json), in parallel with steps 1–2 — data rows.** Add the Area-2 `progression_families` rows and Area-3 `figuration_catalogue` rows (the four no-new-field idioms), plus the `texture_catalogue` profiles and `texture`/progression `SelectTable` rules that select them. Pure JSON; no Rust. (If the lead approves the optional `register_octaves` field for oom-pah/stride, that is a `composition.rs` serde-default addition by the implementer *plus* a one-line `figured_bed` read by the music-theory lane — a small cross-lane coordination; default recommendation is to defer it to Slice 2 and keep Slice 1 lanes fully disjoint.)
4. **Test lane (tests/counterpoint_s30.rs + extensions) — after steps 2 and 3.** Write PT-0..PT-10 and re-assert PT-FREEZE. The hand-built `CounterMelody` section harness (chord_engine.rs 5308–5390) is the template; the data-row tests extend `figuration_s20.rs` and the `generate_chords` net.
5. **Quality gate:** full suite green, `engine_equivalence` 9/9 byte-green, engine.rs sha256 unmoved, the new counterpoint net green. The owner's afternoon listen is a *confidence* pass over a now-test-correct counter-voice — not the acceptance gate.

---

## Appendix — file/line index of cited existing code (read-only)

| Symbol | File | Lines |
|---|---|---|
| `CounterMelody` realize arm | chord_engine.rs | 1703–1792 |
| `pick_counter_pitch` (the scorer to deepen) | chord_engine.rs | 2957–3080 |
| `interval_class` | chord_engine.rs | 2650–2652 |
| `has_parallel_perfects` | chord_engine.rs | 2661–2674 |
| `upper_voice_candidates` | chord_engine.rs | 2627–2645 |
| `motion_dir` / `MotionDir` | chord_engine.rs | 2730–2744 |
| `CONTRARY_BONUS` | chord_engine.rs | 2699 |
| `COUNTER_CEILING` / `FILL_REGISTER_FLOOR` | chord_engine.rs | 2690 / 1190 |
| `counter_candidate_pitches` | chord_engine.rs | 2918–2945 |
| `PhrasePosition` | chord_engine.rs | 507–517 |
| `StepPlan` | chord_engine.rs | 528–545 |
| `realize_step` (public 7-param) | chord_engine.rs | 1055–1062 |
| `figured_bed` (figuration mapper) | chord_engine.rs | 1880–1932 |
| `pick_progression` / `generate_chords` | chord_engine.rs | 119–145 / 170+ |
| hand-built CounterMelody test section | chord_engine.rs | 5308–5390 |
| `FigurationSpec` / `FigurationOnset` | composition.rs | 531–557 |
| `Section` / `StepContext` | composition.rs | 858–899 / 971–984 |
| `progression_families` | mappings.json | 42–46 |
| `figuration_catalogue` | mappings.json | 222–231 |
| `texture_catalogue` / `pad_bed_counter` / `texture` SelectTable | mappings.json | 216–242 |

# Design — S38 Theory Diagnosis (moving home tonic · motif vocabulary + development · craft-side density)

Date: 2026-06-18
Status: DESIGN / DIAGNOSIS ONLY — no `src/*` edited in this session. Authored by the Music Theory Specialist. Three Affect & Aesthetics-adjacent rows in `assets/mappings.json` are *proposed* here for that specialist; the harmonic/voice-leading correctness of every change below is owned here.
Grounding (re-read in the working tree, not trusted from memory): `src/composition.rs` (KeyTempoPlan `:1025`, `home_root_midi = 60` `:1302`, section harmony re-root `:1407–1443`, `pick_archetype` `:1500`, `clamp_variation_slice1` `:1516`, theme seed `:1328–1342`), `src/chord_engine.rs` (`MotifNote` `:2312`, `MotifArchetype`+`contour()` `:2320–2363`, `resolve_motif` `:2379`, `degree_to_pitch` `:2426`, `seat_pc_in_register` `:1282`, `role_pitch` register floors `:1189–1191`, `realize_rhythm` `:1453`, ARTIC window `:1411–1412`, `FILL_REST_ACTIVITY` `:1436`, `figured_bed` `:2206`, `walking_bass` `:2019`, `theme_melody_pitch`/`theme_pitch` `:2472–2549`).
FREEZE anchor honored throughout: `src/engine.rs` is byte-frozen (sha256 `e50c7db1…48261`). Every lever below lands in `composition.rs` (planner), `chord_engine.rs` (craft), or `assets/mappings.json` (data). `engine.rs` is never opened. Critically, the plan-first compose path calls `planner.plan(...)` at `engine.rs:368` and reads `plan.key_tempo.home_root_midi` — the engine's own `config.root_midi = 60` (`engine.rs:206/416`) belongs to the LEGACY non-plan compose body and is NOT the value the shipped path realizes from. So the tonic lever lives entirely at `composition.rs:1302`.

---

## 0. HEADLINE VERDICT (read first)

1. **Every image starts on the same pitch — this is the SMALLEST real fix and it is genuinely safe.** The hardcode at `composition.rs:1302` is the only thing freezing the tonic. Because the realizer re-seats EVERY sounding voice by *pitch class* at a fixed role floor (`seat_pc_in_register`, floors 36/55/67), the absolute register of the bass/fill/melody does **not** move when the tonic moves — only the pitch classes rotate. There is therefore **no register hazard from the tonic value itself**; the only guard-rail is to keep the home pitch class inside a window the existing harmonic builders and `degree_to_pitch` resolve cleanly. **Safe window: home_root_midi ∈ [57, 68]** (A3–G♯4), centered on 60. Detail in §2.
2. **The motif problem is REAL and it is YOUR core, but it is NOT primarily a vocabulary-size problem — it is a SELECTION-collapse + a RHYTHM-flatness problem.** 8 contours exist; `pick_archetype` reaches only 4, and the busy-image early-return makes `Ascent` swallow most photographs. Worse, **every motif note is `dur_steps: 1`** (`resolve_motif:2415`), so even distinct contours arrive as the same undifferentiated even-note stream. Rhythm is the missing variety axis. Fix is three-part: (a) widen selection to all 8, (b) give the motif a *rhythmic profile* (the highest-leverage single change), (c) add real section-to-section *development* beyond Identity/Fragmented. Detail in §3. **This is the highest-leverage finding from the theory lens (§6).**
3. **Density-reversed-too-sparse is correct and has THREE compounding craft causes**, all freeze-compatible: the articulation window's busy-end is too detached, `figured_bed` is capped at the *spec's* onset count (and the default bed is a single block chord — zero inner motion), and `walking_bass`/fill thin out under the conservative density read. Detail in §4.

---

## 1. Why everything sounds the same — the shared root cause across all three findings

All three findings share one diagnosis at the theory level: **the piece is over-governed toward a single safe default and the image-derived knobs that should perturb it are either frozen, collapsed, or pointed at the wrong axis.** The tonic is literally frozen (#1). The contour selection collapses to ~2 shapes and the motif has no rhythm (#2). The density knobs exist but the realizer's mapping makes the *busy* end the *sparse-feeling* end (#3). The pre-S38 architecture was correct to install a "principled default + conditional departure" governor; S38's job is to *let the departures actually fire* without breaking the legality the governor protects.

---

## 2. Finding #1 — moving the home tonic per image

### 2.1 Theory diagnosis (code-grounded)
`home_root_midi = 60` is the ONLY frozen tonal coordinate. Everything downstream is already root-parametric:

- **`home_mode` is already image-derived** (hue→mode `:1294`, then `valence_family_mode` `:1297`). Only the pitch is stuck.
- **Section harmony is already re-rooted** off `home_root_midi + key_offset` (`composition.rs:1418`, `generate_chords(..., section_root_midi, ...)`). The chord builders (`roman_to_chord_complex`, `tonic_triad`, the secondary-dominant / borrowed-iv / ♭VI helpers, all `chord_engine.rs:300–460`) compute every pitch as `root_midi + scale[degree]` — pure transposition. They are root-agnostic by construction.
- **`degree_to_pitch` (`:2426`) is root-agnostic**: it takes `tonic_pc` (a pitch class), indexes the mode scale with `rem_euclid`, and seats via `seat_pc_in_register`. Negative and >6 degrees are handled (the `div_euclid` octave term `:2443`). No degree math is baked to pc 0 / MIDI 60.
- **`key_scheme` is RELATIVE** (`Vec<i8>` offsets, `:1033`) and composes as pure addition onto any home (`section_root_midi` math `:1418`). `relative_offset` (`:1578`) derives ±3 from the mode family, never from the root. So a non-60 home composes correctly.

**The decisive register fact:** the realizer does NOT sound chords at their generated octave. `role_pitch` (`:1217`) and `theme_pitch` (`:2533`) extract the pitch CLASS and re-seat it at the role floor (`seat_pc_in_register :1282` — floors BASS 36 / FILL 55 / MELODY 67). So the *absolute* register of the texture is anchored by the role floors, **independent of the tonic**. Moving the tonic rotates the pitch classes; it does not move the bass down to where it booms or the melody up to where it shrieks. This is why #1 is safe where a naive "just change the root" would be dangerous in an engine that sounded chords at their literal generated octave.

### 2.2 The safe register/octave window + guard-rails
- **Safe home-tonic window: MIDI 57–68 (A3 … G♯4), 12 semitones centered on 60 (C4).** Recommended derivation: map the image's *dominant hue* (or a low-frequency color statistic) onto this 12-semitone window so the tonic becomes a stable identity per image, NOT a per-step wander. One tonic per piece (it is the *home*); excursions are still `key_scheme`'s job.
- **Why this window and not wider:** the only place the tonic's absolute value (not just its class) leaks into pitch is the chord-generation octave *before* re-seating, and the `degree_to_pitch` `+12*octave` term (`:2448`) which is applied *on top of* the seated floor then clamped to `24..=108`. With melody floor 67 + bright lift ≤12 + a motif that can reach degree +7 (≈ +12 semitones via the octave term) you already sit near 67+12+12 = 91, well under 108. A home an octave above 60 (i.e. 72) would push that sum toward 103 and risk top-end clamping (which flattens contour — the `no_inversion_invariant` margin documented at `chord_engine.rs:968`). Keeping the home within ±~½ octave of 60 preserves every existing headroom proof. **Do NOT exceed 68 at the top without re-deriving the 81≪96 / 103≪108 margins.**
- **Mode-scale validity:** all six modes in `degree_to_pitch` (`:2427–2434`) are 7-note arrays indexed by `rem_euclid(7)` and seated by class — every one is valid off any root. No guard needed.
- **Cadence/voice-leading assumptions baked to root 60:** NONE found. The cadence ring (`realize_rhythm :1575`), the pivot realizer (`pivot_chord_events :2593`, which reads `home_root_midi` as a variable `:2612/2727`), and the land-home detector all read the home as a variable. The pivot V is computed as `dest_root_pc + 7` (`chord_engine.rs` pivot doc §2) — pure pc arithmetic, correct off any home.

### 2.3 What needs CARE (the one real subtlety)
The `generate_chords` call clamps `section_root_midi` to `0..=127` (`composition.rs:1419`). With a home near 60 and the menu offsets `{+7,+5,+3,−3}` (`:1414`) the section root stays in a sane octave and re-seating normalizes it anyway. But if a future scheme stacks large positive offsets onto a high home, the *pre-seating* chord octave drifts up; since `role_pitch` re-seats by class this is cosmetically harmless today, **but it means a future "voicing reads the generated octave directly" change would inherit a latent dependency.** Flag for the implementer: keep the home in [57,68] and the invariant "register is set by role floor, not by generated chord octave" stays true. **No pivot/cadence rework is required for #1** (see §5) — the home is a single fixed tonal center per piece; modulation legality is already `key_scheme`'s separately-guarded concern.

---

## 3. Finding #2 — too few distinct motifs (THE core, owned here)

### 3.1 Theory diagnosis — it is selection + rhythm, not (mainly) vocabulary
Three independent collapses stack:

1. **Selection collapse.** `pick_archetype` (`composition.rs:1500`) can only return `{Arch, NeighborTurn, Descent, Ascent}` — and the FIRST line is `if edge_activity >= 0.6 { return Ascent }`. Real photographs run high edge_activity, so a large fraction of images short-circuit to `Ascent` before hue is even consulted. The hue branch then splits the remainder across only 3 shapes. **Heard result: ~2 dominant shapes (Ascent for busy, Arch for the rest).** The 4 expressive contours — `InvertedArch`, `LeapStep`, `Pendulum`, `RisingSequence` — are *dead code in selection*.
2. **Rhythm flatness.** `resolve_motif` hardcodes `dur_steps: 1` for every note (`:2415`). The contour identity is therefore carried ENTIRELY by pitch; two different contours of the same length both arrive as an even stream of equal-length notes. To the ear this erases much of the contour distinction (a `Descent` and an `Arch` over even eighths read more alike than they should). **Rhythm is the missing variety axis** — the prompt's hypothesis is correct.
3. **Sampling smear.** `resolve_motif`'s "hold the final degree for extra steps" rule (`:2404`, `ci = i.min(n_contour-1)`) means any motif longer than its 5–6-note contour ends in a *static repeated tone*. On a complex image (`length_steps` up to 8, `:1339`) the last 2–3 notes are the same pitch held — a built-in dullness.

### 3.2 Vocabulary expansion (which contours, what they buy)
Activate all 8. The unused four each fill a real expressive gap; recommended hue/affect routing (final cuts are Affect & Aesthetics' to tune; the contours and their voice-leading are owned here):

| Archetype | Contour (degrees) | Expressive role | When to select |
|---|---|---|---|
| `InvertedArch` | 4 2 0 2 4 | a *valley* — sinks then lifts; the emotional opposite of Arch | low brightness + mid edge (a "settling" image) |
| `LeapStep` | 0 4 3 2 1 | opening **leap of a 5th** then gap-fill descent — the single most "tune-like / announcing" shape | high `colorfulness`/saturation, bold subject |
| `Pendulum` | 0 4 0 4 0 | insistent tonic↔dominant oscillation; two-zone, hypnotic | high repetition / low complexity, pattern-y image |
| `RisingSequence` | 0 1 2 1 2 3 | a real **motivic sequence** (cell repeated up a step) — the developmental shape | high complexity (it *is* development built into the head) |

Voice-leading constraints any selection MUST respect (these are why the contours are bounded and safe):
- **Largest interval is the 5th in `LeapStep`/`Pendulum`/`InvertedArch`** (degree 0→4). After `resolve_motif` scales by `range/4` (`:2391`), a wide image stretches that to at most a degree-7 span (clamped `:2389`) ≈ an octave — still singable, and `seat_pc_in_register` + the 24..=108 clamp guarantee it never exceeds playable range. **Do not introduce a contour whose native max interval exceeds 4 (a 5th) at native span** — that is the singability floor the `clamp(1,7)` proof rests on.
- A leap must be **stepwise-recovered**: `LeapStep`, `InvertedArch`, and `RisingSequence` all step back after their leap (gap-fill), which is the species-counterpoint rule "a leap is answered by a step in the opposite direction." `Pendulum` is the deliberate exception (its identity IS the unrecovered oscillation) — acceptable because both poles are chord tones (1 and 5).
- Every contour must **begin on a stable degree** (0 or a chord tone) so head-only `Fragmented` (`theme_melody_pitch :2499`) still starts coherently. All 8 do (they start on 0 or 4).

### 3.3 Rhythm as the missing variety axis (HIGHEST-leverage motif change)
Give each archetype a **rhythmic profile** — a short repeating durational pattern the contour is sampled against, replacing the flat `dur_steps: 1`. This is one edit in `resolve_motif` (Music-Theory-owned) plus optional data in `mappings.json`. Concretely, per archetype a small `&'static [u8]` of `dur_steps` cycled across the sampled notes:

- `Arch` → `[2,1,1,2]` (long-short-short-long; the balanced sigh)
- `Descent` → `[1,1,1,1,2]` (even fall into a longer arrival — the resolving cadence-gesture)
- `Ascent` → `[1,1,2]` cycled (a lift that breathes)
- `NeighborTurn` → `[1,1,1,1,2]` (quick turn, held resolution)
- `LeapStep` → `[2,1,1,1,1]` (the leap LANDS long, the gap-fill is quick — this is what makes it sound *announced*)
- `Pendulum` → `[2,2]` (even, weighty oscillation)
- `RisingSequence` → `[1,1,2]` per cell (matches the 3-note cell boundary — the rhythm *articulates* the sequence)
- `InvertedArch` → `[2,1,1,2]`

Constraints owned here: (a) `dur_steps` must stay `>= 1` (the `MotifNote` contract `:2317`); (b) **the sum of a section's motif `dur_steps` must not exceed `step_len`** or the head over-runs the section — `resolve_motif` already takes `length_steps`, so cap by emitting notes until the durational sum reaches `length_steps` rather than emitting exactly `length_steps` notes (this also *fixes the §3.1(3) static-tail smear*, because a longer-duration final note replaces repeated held tones). (c) This stays freeze-safe: `MotifNote.dur_steps` is already a field; the realizer's theme branch (`theme_melody_pitch`) substitutes only PITCH and is rhythm-agnostic, so today the duration is ignored downstream — **the implementer must also thread `dur_steps` into how long the melody holds the theme note** (a `chord_engine.rs` realize-side read, not an `engine.rs` seam change; this is the one piece of follow-on plumbing finding #2 requires and it must be scoped in the build).

### 3.4 Motif DEVELOPMENT (section-to-section variation beyond Identity/Fragmented)
Today only `Identity` (full restate) and `Fragmented` (head-then-rest, `:2499`) exist; the four locked-out `ThemeVariation` members collapse to Identity (`clamp_variation_slice1 :1516`). The motif is stored as scale degrees, which makes the classic development operations **pure functions on the contour data** — all legal by construction:

- **Inversion** — negate every degree (`d → -d`). `degree_to_pitch` already handles negative degrees correctly (`:2442`). An Arch `[0,2,4,2,0]` inverts to `[0,-2,-4,-2,0]` — a valley. Mode-faithful (the inversion is *diatonic*, intervals reshaped by the scale, which is the idiomatic "tonal inversion," not a rigid chromatic one — this is the more musical default).
- **Transposition by `key_offset`** — already free (the motif is key-relative; a modulating section transposes it for nothing). This is the development that an A′ in a new key gives automatically once #1 unfreezes the tonal plane.
- **Augmentation / diminution** — scale `dur_steps` ×2 / ÷2. The S15 comment at `resolve_motif:2411` explicitly reserved this; §3.3's rhythmic profile makes it meaningful (a flat motif can't be augmented audibly).
- **Sequence** — restate the head transposed up/down a step (what `RisingSequence` bakes in; as a *variation* it can apply to any contour).

Recommended slice for THIS build: activate **Inversion** for a Contrast/B section and **Transposition** (free, via #1+key_scheme) for a Return-in-new-key. Defer augmentation to the rhythm-profile follow-on. Constraint: a developed restatement must remain *recognizably derived* — keep the contour's interval-direction signature (inversion preserves it mirrored; transposition preserves it exactly); do NOT randomize, which would break motivic coherence (the §1 governor at the motif level).

---

## 4. Finding #3 — density feels reversed / too sparse (craft-side fixes)

### 4.1 Theory diagnosis — three compounding causes, all in the craft layer
1. **Articulation busy-end too detached.** The continuous curve maps `edge_activity → ARTIC_WINDOW` with busy = `ARTIC_WINDOW_LO = 0.55` (`:1411`, `:1533`). A busy photograph (the common case) lands near 0.55 — notes hold only 55% of their slot, leaving 45% silence per note. The ear reads a busy image as *thin and clipped*, the reverse of intended. The window LOW is too low for the *typical* image.
2. **Default bed has zero inner motion + `figured_bed` is onset-capped.** When `figuration_resolved == None` the Pad arm emits one sustained block chord per step (`:1707`-region block bed) — no subdivision. When a figuration IS present, `figured_bed` emits exactly `spec.onsets.len()` events, truncated to **≤ 4** (`:2223`). So inner-voice activity is either 1 (block) or capped at 4 per step regardless of how busy the image is. There is no path by which a high-activity image *adds* figuration onsets.
3. **Fill rests and conservative bass.** `FILL_REST_ACTIVITY = 0.10` (`:1436`) is well-tuned (it correctly stopped the old "harmony vanishes" bug) and should NOT be loosened. The thinness is NOT coming from over-resting now. The bass, on the default (Sustained/None) arm, emits a single sustained root per step (`:1682`) — correct as a floor, but combined with (1)+(2) the whole texture is: one clipped melody note + one block pad + one sustained bass = sparse.

**Net:** the texture thins because the *busy* signal routes to *shorter notes* (1) and there is *no compensating increase in onset density* anywhere (2). That is the felt reversal.

### 4.2 Craft-side density fixes (freeze-compatible)
- **Raise the articulation busy-floor: `ARTIC_WINDOW_LO 0.55 → 0.62`** (a `chord_engine.rs` const). Below ~0.55 reads as a click; 0.62 keeps detachment audible on truly busy images while closing the per-note silence that makes the texture feel thin. Keep `ARTIC_WINDOW_HI = 1.10`. This is a one-const change; the curve shape is unchanged, the cadence ring (separate, `:1603`) is untouched, and the calm end stays legato. *Falsifiable by ear: busy images should fill, not stutter.*
- **Let figuration onset COUNT scale with activity, within the existing cap.** `figured_bed` is already bounded to 4. Today the *spec* fixes the count. Proposal (Music-Theory + Affect): keep the catalogue's onset list as the MAX, but on a low-activity image use a *prefix* (e.g. 2 of 4 onsets) and on a high-activity image use the full 4. This makes density move the RIGHT direction (busy → more inner onsets) and stays ≤4 by construction, so the bounded-burst safety proof (`:2221`) holds. Implementation is a prefix-length read off `edge_activity`/`ctx.section.density` inside `figured_bed` — a craft-layer edit.
- **Default-bed inner motion.** For sections with NO figuration resolved and `density` above neutral, allow the Pad block bed a single mid-step re-articulation (2 onsets) instead of one held block — a minimal "breathing" of the bed. This must respect the `PAD_OVERLAP_FRAC` overlap cap (`:1447`) so it never stalls the scheduler, and must stay byte-identical on the identity path (`density == 0.5` → 1 onset, no change). This is the same byte-freeze hinge the S29 density nudge already uses (`:1485`).
- **Bass:** leave the default sustained arm alone (it is the floor). Density on the bass is better expressed by *selecting* the Walking pattern (`:1625`) more readily on busy images via `bass_pattern` selection (an Affect/data lever in `mappings.json`), not by editing `walking_bass` itself. `walking_bass`'s `density` arg already drives its onset count correctly (`:2029`).

Constraint owned here: none of these may move a cadence step (the `is_cadence` early return at `:1575` already excludes cadences from all of the above) and none may push a voice out of its register band (all operate on already-seated pitches).

---

## 5. Which findings need a PIVOT / cadence to stay legal

- **#1 (moving the home tonic): NO pivot or cadence rework required.** The home is a single fixed tonal center chosen per *piece*. Choosing C vs E♭ as home does not modulate anything — the whole piece is simply in a different key, and every cadence/voice-leading routine already reads the home as a variable (§2.2). The pivot machinery (`pivot_chord_events :2593`) is concerned with *inter-section* key CHANGES driven by `key_scheme`, which is a separate, already-guarded axis. **The moment a future change couples #1 to a key_scheme that actually modulates AND a non-identity home, the existing pivot guard (scheme `pivot == true`, real offset delta) already arms the V-of-destination preparation** — so legality is preserved automatically; no new cadence is needed for #1 itself.
- **#2 development via Transposition (Return-in-new-key): needs the pivot guard, which already exists.** If a developed restatement lands in a transposed key, that transposition is a `key_scheme` offset and the existing `pivot_chord_events` (V of destination, `dest_root_pc + 7`) prepares it legally. Inversion and augmentation need NO cadence work (they stay in-key). So: only the *transposed* development consumes the pivot path, and that path is built.
- **#3 (density): NO cadence interaction** — every lever is gated to exclude `is_cadence`.

The voice-leading correctness of any key-center move Aesthetics proposes is owned here: the rule is the V-of-destination preparation already specified in the pivot doc; any new excursion offset Aesthetics adds to a `key_scheme` must route through that guard, never as a bare splice.

---

## 6. Highest-leverage finding (theory lens)

**#2, and within it the RHYTHMIC PROFILE (§3.3), is highest-leverage.** Reasoning: #1 is real but its perceptual payoff is "different pieces start in different keys" — valuable for identity, but a listener with relative pitch hears two transposed-but-otherwise-identical pieces as *the same piece*. #3 fixes a texture defect (thinness) but does not add musical *content*. #2 is the only finding that changes what the music SAYS — distinct, rhythmically-differentiated, developed motifs are the difference between "a coherent scan-driven texture" and "a tune." And the single cheapest sub-lever is rhythm: activating the 4 dead contours helps, but giving every motif a durational profile (§3.3) is what makes *any* contour, including the four already active, read as a distinct musical idea rather than an even-note arpeggio. Recommended build order: (1) motif rhythmic profile + fix the static-tail smear [§3.3], (2) widen `pick_archetype` to all 8 + drop the `Ascent` short-circuit [§3.2], (3) the articulation busy-floor const [§4.2], (4) unfreeze the tonic into [57,68] [§2], then (5) Inversion development [§3.4]. Each is independently falsifiable by ear and independently freeze-safe.

---

## 7. mappings.json rows proposed (for Affect & Aesthetics to integrate; harmonic correctness owned here)

These are *proposals* — Affect & Aesthetics owns the selection-cut numbers; this specialist owns that the targets are legal:
- `theme_archetype` SelectTable: extend the current 4-way cut to a hue×affect 8-way that can reach `InvertedArch`/`LeapStep`/`Pendulum`/`RisingSequence` per §3.2. (Replaces the hardcoded `pick_archetype` logic with data, or augments it.)
- `home_tonic` range-map: `dominant_hue → MIDI [57..68]` (per §2.2). Single value per piece.
- Per-archetype `rhythm_profile` arrays (§3.3) — these are *theory data* (durational templates), single-writer here; Affect tunes only which archetype an image gets, not the profile of a given archetype.
- `figuration` onset-prefix policy keyed on activity/density (§4.2) — Affect owns the activity cuts; the onset lists stay ≤4 (correctness here).

End of design note.

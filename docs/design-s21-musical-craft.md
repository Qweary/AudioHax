# Design S21 — Character as Craft + Saliency → Role

**Author role:** Music Theory Specialist (DESIGN ONLY — no source, test, or asset modified by this document).
**Date:** 2026-06-15
**Companions:** the affect design (separate author) owns image → valence/arousal → character SELECTION; the architecture design owns how a plan binds to the engine and how each image property is extracted. This document owns the **musical craft**: (A) what each musical CHARACTER concretely *is* in terms the existing `chord_engine` realizer can already produce, and (B) how image saliency / figure-ground drives musical role PROMINENCE (melody for the salient subject, background for the least-prominent regions).

**Standard:** the project owner has a music-performance degree (trombone) and a working ear. Every preset below is a *coherent* sound a player would recognize, and every mapping carries a perceptual/theoretic reason — not a knob turned for its own sake. Discipline: **a small, coherent vocabulary realized almost entirely from machinery that already exists**, conservative-correct over ambitious-buggy.

**The concrete failure this addresses:** on a bright, chaotic, high-energy image (`example.jpg`), the composer pins `Character::Ballad` and a slow-to-mid tempo for every image, so a maximally energetic painting comes out lifeless. The fix is two-sided: the affect design picks a *richer character* for the high-arousal/high-valence corner, and THIS design makes sure that character has a concrete, joyful, energetic *realization* the existing craft can deliver — plus it gives the salient subject of the image a real melodic voice over a recessive background.

---

## 0. WHAT ALREADY EXISTS (so a character is mostly a preset, not new code)

Before defining characters, the honest inventory of craft a preset can compose from. Everything in this list is **already in `chord_engine.rs` / `composition.rs` / `mappings.json`** and is exercised today; a character preset is overwhelmingly a *selection and biasing* of these, not new machinery.

| Craft layer | Where it lives today | The knob a character turns |
|---|---|---|
| **Diatonic mode** (6 church modes) | `IONIAN`…`AEOLIAN`, `generate_chords` | mode/color tendency — picked from hue, biased major/minor by valence |
| **Harmonic complexity** (triad / 7th / 9th) | `HarmonicComplexity::from_saturation01` | richness — already saturation-driven; a character may floor/ceiling it |
| **Borrowed color** (minor iv, bVI, secondary dominant) | `borrowed_minor_iv` / `flat_submediant` / `secondary_dominant_of` | harmonic-rhythm "spice" density and where it concentrates |
| **Voice leading** (common-tone, ≤P5 cap, no parallel perfects) | `voice_lead_sequence` / `voice_lead_one` / `has_parallel_perfects` | invariant — characters never touch it |
| **Phrase model + cadence at boundary** | `plan_phrases` / `PhrasePosition` / `CadenceStrength` | cadence strength per boundary (already a `Section` field) |
| **Structural velocity floor** (76/88/96) | `plan_phrases` | untouched floor; the contour rides on top |
| **Dynamic contour** (level gain, messa-di-voce, metric accent, taper, per-role bias) | `realize_velocity` | dynamic posture — per-character `level` and accent weights (plan-supplied scalar) |
| **Continuous articulation curve** + **per-character ARTIC bias seam** | `realize_rhythm` `curve_frac` + `BALLAD_ARTIC_BIAS` | **articulation bias is ALREADY a seam** — Ballad=1.0 no-op today; other characters set their own multiplier into the same `0.55..1.10` window |
| **Rhythm-pattern bands** (sustained / dotted / syncopated / arpeggio) gated by `edge_activity` | `realize_rhythm` melody arm | rhythmic-figuration density — a character shifts the band thresholds |
| **Harmonic-rhythm acceleration + ritardando** | `pre_cadence`, `RITARDANDO_FACTOR` | already wired; characters lean on it more or less |
| **Orchestration roles** (Bass / HarmonicFill / Melody / Pad / CounterMelody) | `OrchestralRole`, `assign_role`, `role_pitch` | texture — which roles sound, via the `OrchestrationProfile.layers` data list |
| **Held Pad bed (root-less inner voicing)** | Pad arm of `realize_rhythm` | sustained accompaniment density (`pad_voices`) |
| **Accompaniment figuration (Alberti / broken-chord)** | `figured_bed` + `FigurationSpec` catalogue | figuration bias — a character names a `figuration` id on its profile |
| **Real counter-melody** (contrary-motion, chord-tone, no parallel perfects) | CounterMelody arm + helpers | a second independent line — a character may request it via the profile |
| **Returning theme** (motif resolve + replay, fragment-in-B) | `resolve_motif` / `theme_melody_pitch` | untouched; characters give the theme its rhythmic identity, not its pitch |
| **Register bands** (Bass C2 / Fill G3 / Melody G4) | `*_REGISTER_FLOOR`, `seat_pc_in_register` | invariant separation; saliency (Part B) widens it, never collapses it |
| **Closed `Character` enum** (10 variants) | `composition::Character` | the variant set already exists — this design assigns each a concrete meaning |
| **Closed `Meter` enum** (Four4/Three4/Six8/Two4) | `composition::Meter` | meter feel — bound to character |
| **`OrchestrationProfile` / `texture` SelectTable** conditioned on saliency knobs | `composition.rs` + `mappings.json` | the saliency → texture selection seam (Part B rides this) |
| **Saliency knobs already on the input** | `ImageUnderstanding.subject_energy / foreground_energy / background_energy / fg_bg_contrast / subject_size` | the figure-ground signal Part B reads |

**The single most important observation:** the `Character` enum already carries `Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt, Gigue` and `parse_character` already maps all ten names. The realizer already exposes a **per-character articulation-bias seam** (`BALLAD_ARTIC_BIAS`, explicitly documented as "the documented seam the later characters … ride on"). So a character preset is, almost entirely, a **data row** (a `CharacterOverlay`-style scalar bundle the plan attaches) plus a `texture`/`figuration` selection — *not* a realizer rewrite. The one genuinely new behavioral lever any character needs is **plan-supplied scalars threaded into `realize_velocity` / `realize_rhythm` defaulting to identity** so the byte-frozen default reproduces today exactly (see §A.9 and Part C).

---

## PART A — CHARACTER AS CRAFT

### A.1 The recommended set: SIX shipped, with an explicit ENERGETIC character added

The roadmap's reserve list was Ballad / Hymn / Nocturne / Drone / March / Lament. Assessed against the failing case and the affect plane, that set has a hole exactly where `example.jpg` lives: **the high-arousal / high-valence corner has no home.** March is high-arousal but *not* joyful — it is firm, terraced, martial; a bright chaotic painting rendered as a march would read as grim, not exuberant. Hymn is consonant and warm but static and slow. None of the six is the musical signature the affect research names for the bright-energetic-vivid image: **fast + major/consonant + bright register + dense onsets + buoyant articulation.**

**Recommendation: yes, add an explicitly energetic/joyful character.** Name it **Jubilee** (a bright, fast, dancing character — the affect literature's "high arousal + high valence" signature). It is the direct cure for the `example.jpg` failure: it is the character the affect design selects when arousal is high AND valence is high, and it has a concrete realization (below) that the existing craft delivers without new machinery. The `Character` enum does not yet carry `Jubilee`; adding one closed variant + one `parse_character` arm is the only enum edit this design requests, and it composes with the existing `Lilt`/`Gigue`/`Scherzo`/`Waltz` variants the enum already exposes for the dance/playful neighborhood.

**The six (+1) shipped characters and the role each plays on the valence×arousal plane:**

```
        HIGH AROUSAL
            │
   March ───┼─── Jubilee            (March: high-A, neutral/low-V, martial)
  (driving) │   (fast, bright,      (Jubilee: high-A, high-V — THE energetic/joyful corner)
            │    JOYFUL)
            │
LOW ────────┼──────── HIGH  VALENCE
            │
  Lament ───┼─── Hymn               (Lament: low-A, low-V, dark/slow)
 (dirge)    │   (warm, devotional)  (Hymn: low/mid-A, high-V, consonant, stately)
            │
  Drone ────┼─── Ballad / Nocturne  (Drone: very low-A, V-neutral, static)
 (static)   │   (Ballad DEFAULT: low-mid-A, mild-V; Nocturne: low-A, gentle, intimate)
            │
        LOW AROUSAL
```

Ballad remains the **default** (the safe, pleasant center and the byte-freeze anchor). The affect design's selection logic maps an image's (valence, arousal) to one of these; this design guarantees each has a concrete, defensible sound.

### A.2 Reading the table (the eleven craft dimensions)

Each preset below fixes the same eleven dimensions, all expressed in the existing craft vocabulary:

1. **Meter feel** — `Meter` variant (bound to character, never read independently from the image — keeps the vocabulary coherent: no "3/4 march").
2. **Tempo range (BPM)** — a window that **clamps/centers** the S13 brightness→BPM result. Character wins the *category*; brightness wins the *position within it*. (Today only the `BALLAD_BPM_MIN/MAX` 56–96 window exists; each character supplies its own pair.)
3. **Mode / harmonic-color tendency** — which modes the character prefers and the **valence-driven major/minor lean** (the affect research's one load-bearing mode mapping). Hue still seeds the specific mode; the character biases toward the bright (Ionian/Lydian/Mixolydian) or dark (Aeolian/Dorian/Phrygian) family.
4. **Harmonic rhythm** — how often chords change relative to the beat, and how much borrowed color (minor iv / bVI / secondary dominant) the section concentrates.
5. **Phrase-model bias** — whether phrases lean to the 4-step (tight, dance-like) or 8-step (broad, singing) end of `PHRASE_LENGTHS`, and how strongly cadences are differentiated.
6. **Orchestration-role profile** — which of Bass / Pad / HarmonicFill / Melody / CounterMelody are active and how dense, expressed as an `OrchestrationProfile.layers` list (a `texture_catalogue` row).
7. **Articulation bias** — the per-character multiplier into the existing `0.55..1.10` non-cadence window (the `ARTIC_BIAS` seam): toward 1.10 = legato/singing, toward 0.55 = detached/crisp.
8. **Dynamic shape** — bias on `realize_velocity` (overall level + how pronounced the messa-di-voce arch and metric accents are), as a plan-supplied scalar.
9. **Rhythmic figuration bias** — how readily the melody subdivides (shifting the `edge_activity` band thresholds in `realize_rhythm`) and whether the Pad runs a block bed or an animated figure (`figuration` id).
10. **Form bias** — which macro-forms the character is at home in (a soft preference the form SelectTable can honor; never a hard coupling).
11. **Position on the valence×arousal plane** — so it composes with the affect design's selection.

### A.3 BALLAD *(DEFAULT — the byte-freeze anchor)*

- **Meter:** 4/4. **Tempo:** 56–96 BPM (the existing `BALLAD_BPM_MIN/MAX`). **Mode/color:** home mode from hue, mild valence lean; full 7th/9th richness welcome.
- **Harmonic rhythm:** slow — roughly one chord per measure; borrowed color sparse and only where the image earns it.
- **Phrase bias:** broad (8-step phrases preferred); cadence strengths as the form template states.
- **Orchestration:** full ensemble, melody-led, sustained inner fill — the `pad_bed` / `pad_bed_counter` profiles already selected by the `texture` table. (Sustained accompaniment is exactly what `pad_bed` builds.)
- **Articulation:** strongly legato — `ARTIC_BIAS = 1.0` centering the curve toward the `ARTIC_WINDOW_HI` (1.10) ceiling so the line sings across the bar. **This is the current behavior; Ballad's bias is identity.**
- **Dynamics:** gentle messa-di-voce arches, wide-but-soft. The current `realize_velocity` contour *is* the Ballad posture.
- **Figuration:** block bed (no Alberti) by default; melody stays in the sustained/dotted bands.
- **Form bias:** rounded binary (the form default), ternary.
- **Plane:** low-mid arousal, mildly positive valence — the safe pleasant center.
- **Reuses existing craft:** ENTIRELY. Ballad is the no-op preset; **this is the identity default the byte-freeze pins** (§Part C / §7 of the architecture design). Nothing in Ballad is new craft.

### A.4 HYMN

- **Meter:** 4/4. **Tempo:** 66–88 BPM (stately, walking). **Mode/color:** strongly major-leaning (Ionian, occasionally Mixolydian); high consonance — *suppress* the secondary-dominant chromaticism, keep borrowed color to the occasional plagal/bVI warmth. High valence is the point.
- **Harmonic rhythm:** **chordal/homophonic** — one chord per beat or per two beats, all voices moving together. This is the hymn's defining texture.
- **Phrase bias:** square, balanced (4- and 8-step phrases that pair into clear periods); cadences strong and frequent (every phrase closes firmly — IAC/PAC, plagal "amen" at section ends via the existing `CadenceStrength::Plagal`).
- **Orchestration:** **all roles move as block chords** — Bass + HarmonicFill + Melody in rhythmic lockstep, Pad optional and thin. The melody is the top of the chord, not an independent line (so the theme is carried as the chordal soprano).
- **Articulation:** legato but *weighted* — `ARTIC_BIAS ≈ 1.0`, slightly detached at chord changes for clarity (not the Ballad's overlap). Toward the upper-middle of the window (~0.95–1.0 effective).
- **Dynamics:** broad terraced dynamics, even across the chord; little messa-di-voce (the hymn is not rubato — reduce the swell scalar). Strong on phrase downbeats.
- **Figuration:** none — the hymn is sustained block harmony; the figured bed is explicitly *off*.
- **Form bias:** strophic / ternary (verse-like repetition).
- **Plane:** low/mid arousal, high valence — devotional, warm, stable.
- **Reuses vs new:** **reuses** the block-chord realization (the current default before the Pad/figuration layers), `Plagal` cadence, mode major-lean. The one *new* lever is **suppressing** the saturation-driven chromaticism and the per-step messa-di-voce swell — both expressible as plan-supplied scalars (a "chromaticism ceiling" and a swell-scale of ~0), no realizer rewrite.

### A.5 NOCTURNE

- **Meter:** 4/4 (occasionally 6/8 for a flowing one). **Tempo:** 52–72 BPM (slow, intimate — overlaps Ballad's low end). **Mode/color:** minor-leaning (Aeolian/Dorian) or warm major; rich 7ths/9ths welcome; *espressivo* chromaticism allowed in the inner voices.
- **Harmonic rhythm:** slow harmony, **fast accompaniment** — the defining nocturne split: a sustained/slow chord change under a continuously moving **broken-chord (Alberti/arpeggiated) accompaniment**. This is exactly what the `figured_bed` + `alberti` figuration row builds.
- **Phrase bias:** long, asymmetrical singing phrases (8-step); rubato-feel via the ritardando/messa-di-voce already present.
- **Orchestration:** a singing **Melody** over a **figured Pad** (the `pad_figured` profile that already exists, naming `figuration: "alberti"`), thin or absent HarmonicFill, a grounding Bass. A CounterMelody is welcome on quieter sections.
- **Articulation:** very legato melody (`ARTIC_BIAS → 1.10`), the accompaniment continuously connected (the figured bed's `hold_frac` toward legato).
- **Dynamics:** pronounced messa-di-voce (increase the swell scalar above Ballad); intimate overall level (lower `level_gain` center).
- **Figuration:** **`alberti` / broken-chord ON** — this is the character that most makes the S20 figuration layer a *virtue*.
- **Form bias:** ternary (ABA — a contrasting middle), through-composed reserve.
- **Plane:** low arousal, valence-neutral-to-positive — gentle, lyrical, nocturnal.
- **Reuses vs new:** **almost entirely reuses** — `pad_figured` + `alberti` already exist in the catalogue; Nocturne is the character that *selects* them macro-wide (today the `texture` table selects `pad_figured` only on a busy-subject heuristic). The new lever is making "this character implies the figured accompaniment profile" a plan default + a stronger swell scalar. No new realizer code.

### A.6 DRONE

- **Meter:** free / 4/4 (meter barely felt). **Tempo:** very slow (48–60 BPM) or effectively beatless. **Mode/color:** modal, static — one mode, often Dorian/Aeolian/Mixolydian, *no* functional progression; harmony is a **pedal/tonic prolongation** with color shifting above a held root.
- **Harmonic rhythm:** **near-zero** — the bass holds a pedal (the tonic, or tonic–fifth open drone) for long spans; upper voices add/remove color tones over it. The existing held-run / held-chord machinery in the counter-melody arm already handles "the chord doesn't change for many steps."
- **Phrase bias:** weak/absent cadential articulation — the section breathes rather than cadences (favor `Half`/`Imperfect`, avoid strong PAC mid-piece).
- **Orchestration:** a **sustained Pad bed** dominating, a pedal Bass, a slow Melody floating above. This is the `pad_bed` profile pushed to its sustained extreme (`pad_voices` high, no figuration).
- **Articulation:** maximally legato/overlapping (`ARTIC_BIAS → 1.10`, the Pad's `PAD_OVERLAP_FRAC` continuous-bed behavior).
- **Dynamics:** flat, narrow dynamic range; minimal accent (reduce metric-accent and swell scalars toward 0) — the drone is *static* by design.
- **Figuration:** none (animation would break the stasis); occasionally a single slow inner CounterMelody as the only motion.
- **Form bias:** through-composed reserve (no return needed when there is no departure); single-section.
- **Plane:** very low arousal, valence-neutral — meditative, ambient.
- **Reuses vs new:** **reuses** the Pad bed and the held-chord counter behavior. New lever: a **pedal-bass / static-harmony plan mode** (the planner hands the section a one- or two-chord "progression" and a long step span). That is a *plan* choice, not realizer code — `generate_chords` already accepts whatever progression it is given, and `plan_phrases` tolerates a short repeated chord set. The realizer is untouched.

### A.7 MARCH

- **Meter:** 4/4 (or 2/4 — both already in the `Meter` enum). **Tempo:** 96–120 BPM. **Mode/color:** major or minor (martial works in both); moderate chromaticism (secondary dominants drive well here); harmony firm and functional.
- **Harmonic rhythm:** regular — chord changes on strong beats (1 and 3 in 4/4); the bass anchors those.
- **Phrase bias:** square, tight (4-step phrases = one measure each); clear, frequent cadences; terraced sections.
- **Orchestration:** **Bass prominent on beats 1 & 3, Melody subdividing (arpeggio/eighths), HarmonicFill steady.** The per-beat bass placement is the meter binding (the architecture design's `role_beat_masks`); until that lands, the existing role split with a denser melody approximates it.
- **Articulation:** **detached / marcato** — `ARTIC_BIAS → 0.55` (the staccato end of the window). This is the clearest use of the ARTIC seam in the opposite direction from Ballad.
- **Dynamics:** firm, terraced; strong accents on beats 1 & 3 (increase the metric-accent scalar). Little messa-di-voce.
- **Figuration:** none on the bed; the *melody* subdivides instead (shift the rhythm bands so the melody arpeggiates readily — lower the arpeggio threshold).
- **Form bias:** rounded binary, AABA (trio-and-return).
- **Plane:** high arousal, neutral-to-low valence — driving, firm, martial. **Not** the joyful corner — that is Jubilee.
- **Reuses vs new:** **reuses** the rhythm bands, the `ARTIC_BIAS` seam (pushed to 0.55), and the velocity accent. The genuinely new craft is **per-beat role gating** (bass on 1 & 3 only) — but that is the *meter* mechanism the architecture design owns (a beat-mask threaded into `realize_rhythm`), not character-specific code. March without the beat-mask still reads as a march via tempo + marcato articulation + dense melody; the beat-mask sharpens it.

### A.8 LAMENT

- **Meter:** 4/4 (or 6/8 for a flowing dirge). **Tempo:** 48–66 BPM (very slow). **Mode/color:** **minor — Aeolian / Phrygian / Dorian**; the borrowed minor iv and falling lines are central; suspensions and the descending tetrachord ("lament bass") are the idiom.
- **Harmonic rhythm:** slow, with **descending bass motion** and frequent suspensions/appoggiaturas (non-chord tones resolving down — the existing voice-leading common-tone retention can hold a tone over a chord change to create the suspension).
- **Phrase bias:** long, sighing phrases; **exaggerated phrase-end ritardando** (lean on `RITARDANDO_FACTOR`); deceptive cadences delay resolution (the `CadenceStrength::Deceptive` already in the enum).
- **Orchestration:** Melody + Bass primary, HarmonicFill **thin or resting** (the rest-as-gesture already in the fill arm fits here), Pad sustained and low.
- **Articulation:** legato, weighted, with the exaggerated ritardando at phrase ends.
- **Dynamics:** dark, narrow-and-low; **descending dynamic shapes** (a diminuendo bias — invert/weight the messa-di-voce so phrases fall away rather than arch).
- **Figuration:** none (a dirge is not animated); slow falling inner CounterMelody welcome.
- **Form bias:** ternary, theme-and-variations (a passacaglia/ground-bass lament is theme-and-variations over a repeating bass — a natural fit).
- **Plane:** low arousal, low valence — somber, grieving.
- **Reuses vs new:** **reuses** the minor modes, borrowed minor iv, deceptive cadence, ritardando, fill rest-as-gesture. New lever: a **descending-dynamic bias** (a sign flip / weighting on the messa-di-voce term) and a **descending-bass plan preference** — the first is a plan scalar on `realize_velocity`, the second a planner progression choice. No realizer rewrite.

### A.9 JUBILEE *(the added energetic/joyful character — the `example.jpg` cure)*

- **Meter:** 4/4 or **6/8 (felt in 2, lilting)** — both in the enum; 6/8 gives the dancing buoyancy. **Tempo:** **108–152 BPM (fast)** — this is the character that finally pushes tempo past the ballad ceiling the affect research named as the proximate cause of the failure. Brightness still positions within the window; the *window itself* is fast.
- **Mode/color:** **major / bright modes — Ionian, Lydian, Mixolydian**; high valence ⇒ major, high consonance with *bright* color (add9/6 sonorities, the Lydian #4 sparkle). Rich 7ths/9ths from the high saturation a vivid image carries — exactly what `HarmonicComplexity` already gives a saturated image. Secondary dominants drive forward motion (the busy image's high `edge_activity` already fires `secondary_dominant_of`).
- **Harmonic rhythm:** lively but grounded — chord changes on strong beats with frequent applied dominants pushing ahead; the bright bVI/IV color used as *lift*, not shadow.
- **Phrase bias:** **dance-square 4-step phrases**, antecedent/consequent periods with bright half→PAC pairing; the energy comes from rhythmic drive, not phrase asymmetry.
- **Orchestration:** **full, bright, busy** — Melody high and active, a **figured/broken-chord Pad** (Alberti or a faster broken pattern) carrying perpetual motion, Bass buoyant, a CounterMelody welcome for sparkle. This is the `pad_figured` profile at high density (`density` toward 0.62+).
- **Articulation:** **light and crisp but not heavy** — `ARTIC_BIAS ≈ 0.65–0.75` (toward the detached end, but not the full marcato 0.55 of March; a dancing détaché, not a stomp). Short, bouncing notes are the joy cue — the affect research's "high arousal → staccato."
- **Dynamics:** **bright and forward** — raise the overall `level_gain` center, lively metric accents, buoyant but not heavy; an *upward* energy (the opposite of Lament's fall).
- **Rhythmic figuration:** **dense** — shift the `edge_activity` rhythm-band thresholds **down** so the melody readily arpeggiates/syncopates (the busy `example.jpg` then subdivides instead of sustaining); the Pad runs an animated figure. This is the single most audible difference from Ballad on the same image.
- **Form bias:** rounded binary, theme-and-variations (a busy image's complexity → variations, which the form table already selects on high complexity+edge), rondo (reserve).
- **Plane:** **high arousal + high valence** — the corner no other character occupies; the joyful, energetic image's home.
- **Reuses vs new:** **reuses** every layer — the fast tempo window (same clamp mechanism as Ballad's, different numbers), the `ARTIC_BIAS` seam (toward detached), the rhythm bands (thresholds shifted down), `pad_figured`/`alberti` figuration, the saturation-driven 7th/9th richness, the edge-driven secondary dominants. The **only** genuinely new piece is the **fast tempo window** (which requires the per-character BPM-window mechanism — a data pair, identical in kind to `BALLAD_BPM_MIN/MAX`) and the **enum variant `Jubilee`** + its `parse_character` arm. No new realizer function. **This is the load-bearing claim of Part A: the energetic/joyful sound the failing image needs is fully realizable from existing craft once the character is selected and its scalar bundle (tempo window, articulation bias, rhythm-band shift, dynamic level, figuration on) is applied.**

### A.10 The character preset as a data bundle (the contract to the architecture design)

Every preset above is, concretely, this scalar/selection bundle the plan attaches per section (the architecture design's `CharacterOverlay`, here named in musical terms). **All scalars default to the Ballad identity so the byte-freeze default reproduces today exactly:**

```
CharacterPreset
  meter:              Meter            // bound to character (Four4 default)
  bpm_window:         (f32, f32)       // clamp/center the brightness→BPM result; Ballad = (56, 96)
  mode_valence_lean:  f32              // -1 dark/minor .. +1 bright/major; biases the hue mode pick; Ballad ≈ 0
  chromaticism_scale: f32              // multiplier on borrowed-color/secondary-dominant concentration; Ballad = 1.0
  artic_bias:         f32              // the EXISTING ARTIC seam multiplier into 0.55..1.10; Ballad = 1.0
  rhythm_band_shift:  f32              // shifts realize_rhythm edge_activity band cutoffs; Ballad = 0.0 (no shift)
  dynamic_level_bias: f32              // additive on realize_velocity level center; Ballad = 0.0
  swell_scale:        f32              // multiplier on the messa-di-voce term; Ballad = 1.0
  accent_scale:       f32              // multiplier on the metric-accent term; Ballad = 1.0
  texture_id:         Option<String>   // an OrchestrationProfile id this character implies (pad_bed / pad_figured / …); Ballad = None → table default
  figuration_id:      Option<String>   // overrides/sets the profile's figuration; Ballad = None
  phrase_len_bias:    PhraseLenBias    // prefer 4-step (dance) vs 8-step (singing); Ballad = Broad
```

**Identity vector (Ballad) = (Four4, (56,96), 0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, None, None, Broad)** — and that vector, applied, is byte-for-byte today's behavior. This is the mechanism by which "a character is mostly a parameter preset" is literally true, and by which the freeze stays safe (§Part C).

---

## PART B — SALIENCY → MUSICAL ROLE SYSTEM

The owner's S19 words, addressed directly: *"the most prevalent subject(s) may play more of a role in the melody, whereas the least prominent layer may play a larger role in the background."* This is well-grounded — the affect research (Saliency → Musical Role) anchors it in **figure-ground as a shared visual/auditory organization** (Bregman) and **high-voice superiority** (Trainor et al.): the melody *is* the auditory figure, and the high voice is the most salient one. It is a *design heuristic*, not a proven law, and is documented as such.

### B.1 The principle: prominence reweights an existing role system; it does NOT invent roles

The realizer already has the five roles (Bass / HarmonicFill / Melody / Pad / CounterMelody) and assigns them by ensemble index via `assign_role`, optionally overridden by an `OrchestrationProfile.layers` list. **Saliency does not add a new role taxonomy and does not change which instrument is which role.** It does two things, both *additive on top of* the existing system:

1. **Selection (already wired):** the `texture` SelectTable already reads the saliency knobs (`subject_energy`, `foreground_energy`, `fg_bg_contrast`) and picks a richer `OrchestrationProfile` when a real figure/ground stratification exists — e.g. it selects `pad_bed_counter` (adds a CounterMelody) on a busy foreground with a real subject, and `pad_figured` on a busy salient subject. **This is the saliency → which-roles-are-active seam, and it exists today.** Part B's first move is to *use it more fully*: map saliency **tiers** to profiles (below).

2. **Prominence reweighting (the new, additive scalar layer):** a per-image **prominence vector** that scales the *degree* to which the foreground (melody) stands out from the background (pad/fill) — in velocity, register separation, rhythmic activity, articulation, and voice count. This rides on top of `role_pitch` / `realize_velocity` / `realize_rhythm` as plan-supplied scalars that **default to identity** (so a no-subject / flat image reproduces today's balance and the freeze stays safe).

### B.2 The saliency signal (already on the input)

`ImageUnderstanding` already carries the figure-ground signal Part B needs — no new extraction is required for the core behavior:

- `subject_size` — how much of the frame the salient subject occupies.
- `subject_energy` — edge activity *within* the salient subject region.
- `foreground_energy` — energy in the non-subject central band.
- `background_energy` — energy in the corner/border cells minus the subject.
- `fg_bg_contrast` — how different the subject is from its surround (the "subject-pop" proxy).

These define a **prominence ratio** the plan computes once (a planner-side derivation, not a realizer read):

```
subject_prominence = clamp( fg_bg_contrast * lerp(0.5, 1.0, subject_energy), 0, 1 )
background_recession = clamp( 1 - background_energy, 0, 1 )    // a quiet background recedes more
```

`subject_prominence` near 0 means "no clear figure" — a flat/abstract image, where the system must degrade to today's balanced texture (the freeze case). Near 1 means "a strong, distinct, energetic subject" — push the foreground forward and the background back.

### B.3 Saliency tier → orchestration profile (which roles each layer gets)

Map `subject_prominence` to a profile via the existing `texture` SelectTable (extend its rules; the catalogue rows already exist or are trivial data additions):

| Tier | Condition | Profile (existing/near-existing) | What each layer gets |
|---|---|---|---|
| **No figure** | `subject_prominence < 0.20` | `identity` or `pad_bed` | today's balanced texture — the freeze-safe default; melody = top chord tone, fill sustains. |
| **Mild figure** | `0.20 ≤ prom < 0.45` | `pad_bed` | melody distinct over a sustained pad; the background recedes mildly (velocity/register scalars near identity). |
| **Clear figure** | `0.45 ≤ prom < 0.70`, real contrast | `pad_bed_counter` *(exists)* | melody foregrounded; **the background gains an inner CounterMelody** so the accompaniment is a *living* bed, not a static block — but recessive. |
| **Strong, busy figure** | `prom ≥ 0.70` AND `subject_energy` high | `pad_figured` *(exists)* | melody high, active, and loud; the **background runs an animated (Alberti) figure** so the foreground/background distinction is maximal — the subject *sings*, the background *shimmers underneath*. |

This is exactly the family the current `texture` table already gestures at (it selects `pad_figured` on busy-subject + contrast, `pad_bed_counter` on busy-foreground + contrast). Part B **regularizes those ad-hoc rules into a monotone prominence ladder** so the relationship "more salient subject → more foregrounded melody + more recessive, more interesting background" is systematic, not two special cases.

### B.4 Prominence → the five concrete role-prominence translations

For each music dimension the owner named, here is exactly how `subject_prominence` (foreground) and `background_recession` (background) translate, and how it rides on the existing role behavior. **All are scalars that default to identity at `prominence = 0`** so the no-subject image is unchanged.

**1. Relative velocity (dynamic separation).** The melody role already gets `+2` in `realize_velocity` and the Pad `-3`. Saliency *widens that existing gap*:
- Melody velocity bias: `+2 + round(subject_prominence * 6)` → up to ~+8 over the floor on a strong subject (the foreground gets louder).
- Pad/HarmonicFill velocity bias: `-3 - round(background_recession * 4)` → down to ~-7 (the background recedes further).
- The structural floor (76/88/96) and the cadence exemption are untouched; this only widens the per-role bias already present. **At `prominence = 0` the biases are exactly today's +2 / -3.**

**2. Register separation.** The role register floors (Bass C2 / Fill G3 / Melody G4) already separate the layers. Saliency *widens the melody/accompaniment gap* without collapsing it:
- Melody register floor: lift by up to `+5` semitones on a strong subject (the foreground rises, reinforcing high-voice superiority) — applied as a bias on `MELODY_REGISTER_FLOOR`'s effective value, clamped exactly as today (≤96).
- Pad/fill register: unchanged (the bed stays in the fill band G3); widening is achieved by lifting the melody, not lowering the bed (lowering would risk colliding with the bass). **Invariant preserved:** Bass < bed < melody always holds (Part C). At `prominence = 0`, no lift — today's `MELODY_REGISTER_FLOOR = G4`.

**3. Rhythmic activity.** The melody arm's `edge_activity` band cutoffs select sustained/dotted/syncopated/arpeggio. Saliency lets the **subject's own energy** (`subject_energy`), not just whole-image edge, drive the melody's activity:
- The melody's effective `edge_activity` is nudged toward `max(edge_activity, subject_energy)` so a busy *subject* makes the *melody* active even if the whole image is calm — the foreground gets the rhythmic freedom the owner asked for ("more rhythmic freedom, wider range, melodic independence").
- The background (Pad/fill) stays *less* active: its effective activity is nudged toward `background_energy` (typically lower), keeping it sustained/recessive. The fill's existing "sustained, supports never competes" behavior is reinforced, not replaced.

**4. Articulation.** The foreground gets more articulative freedom, the background stays connected:
- Melody articulation: rides the character `ARTIC_BIAS` (Part A) — saliency does not override the character, it only ensures the melody is the role that *carries* the character's articulation identity.
- Background: the Pad/fill stays toward the legato end (the existing `curve_frac.max(ARTIC_WINDOW_LO)` fill behavior and the `PAD_OVERLAP_FRAC` continuous bed) — recessive and connected, never competing for attention with crisp attacks.

**5. Voice count per layer.** The owner's "least prominent layer plays a larger role in the background" maps to **how many voices the bed holds**:
- A recessive, low-energy background → the Pad holds *more* tones (`pad_voices` higher, a fuller sustained bed — the background is "larger" in the sense of fuller and more enveloping, but quieter).
- A strong, distinct subject → the melody is a *single* independent line (one voice, maximal independence), and a CounterMelody may be *added* (via the profile) so the texture is melody + counter + bed, the foreground clearly leading.
- This is set by the **profile selection** in B.3 (`pad_bed` vs `pad_bed_counter` vs `pad_figured`, each with its `pad_voices` and `layers` list), so voice-count-per-layer is a *data* choice already expressible in the `texture_catalogue`.

### B.5 Interaction with the figured bed (S20) and counter-melody (S18) already built

- **Figured bed:** when saliency selects `pad_figured` (strong busy subject), the background's animation *is* the S20 figured bed — already built and chord-tone-bounded to the fill band. Saliency does not touch `figured_bed`; it only *selects the profile that activates it*. The background "playing a larger role" while staying recessive is precisely a fuller, animated-but-quiet bed.
- **Counter-melody:** when saliency selects `pad_bed_counter` (clear figure), the background gains the S18 real CounterMelody — a second moving line that makes the accompaniment alive without stealing the foreground (it sits below `COUNTER_CEILING = G4`, the melody floor, by construction). Saliency selects it; it does not modify the counter's contrary-motion/no-parallel-perfects craft.
- **No double-counting:** prominence reweighting (B.4) and profile selection (B.3) compose cleanly — selection decides *which roles exist*, reweighting decides *how far apart they sit*. Both default to identity at `prominence = 0`.

### B.6 The contract: a prominence vector the plan computes, the realizer reads as scalars

```
SaliencyProminence            // computed once per plan from ImageUnderstanding (planner side)
  subject_prominence:    f32   // 0..1; foreground strength (B.2)
  background_recession:  f32   // 0..1; how far the background recedes (B.2)
  // → selects the OrchestrationProfile via the texture SelectTable (B.3)
  // → supplies identity-defaulting bias scalars to role velocity / register / activity (B.4)
```

Threaded into `realize_velocity` / `realize_rhythm` / `role_pitch` exactly like the character scalars (Part A.10) — additive, identity at 0, never altering the cadence branch or the structural floor. **At `subject_prominence = 0` (no figure / abstract image), every bias is identity and the realizer is byte-identical to today** — the freeze case (Part C).

---

## PART C — INVARIANTS & TESTABILITY

### C.1 Musical invariants any build MUST preserve

These are the non-negotiables. A character preset or saliency reweighting that breaks one is a defect, not a trade-off.

1. **Voice-leading limits.** Upper voices move ≤ a perfect fifth (`MAX_UPPER_VOICE_MOTION = 7`); no parallel perfect fifths/octaves between any voice pair across a chord change (`has_parallel_perfects`). Characters and saliency change *dynamics/register/rhythm*, never the voice-leading search — these stay intact by construction (no preset touches `voice_lead_one`).
2. **No unison collapse.** No two upper voices on the same MIDI note (`upper_voices_well_spaced`, `MIN_UPPER_VOICE_SPACING = 1`). The Pad bed's de-dup and the counter's no-unison-double (`COUNTER_UNISON_PENALTY`) preserve this; a fuller saliency-driven bed must keep de-duping.
3. **Register separation / no foreground-background inversion.** Bass < bed/fill < melody in sounding pitch, always. Saliency *widens* this gap (lifts melody, never lowers the bed into the bass); the clamp to 24..=108 and the role floors guarantee it. **A build that lets a louder/higher background overtake the melody, or a melody lift that pushes it out of the 24..=108 band, breaks this.**
4. **Cadence at boundary.** Section/phrase boundaries cadence (`plan_phrases` stamps `HalfCadence`/`PerfectAuthenticCadence` at boundaries); the structural close is the strongest cadence. Characters set cadence *strength* per boundary but never move a cadence off a boundary or into the interior.
5. **The cadence ring is byte-stable.** The cadence branch (`sustained(0, step_ms, LEGATO_FRAC)`, the 240 ms / 1.20-cap ring) is untouched by any character or saliency scalar — biases act on **non-cadence** steps only. This protects the cadence golden.
6. **S13 diversity inequalities still hold.** Two distinct images still differ in ≥3 of {tempo, mean note-length, onset-count distribution, max chord-tone count, mode/mixture}. Characters *add* a dimension (character identity itself), they must not *erase* an existing one — e.g. a character must not flatten tempo to a constant (its BPM window still lets brightness vary within it).
7. **Returning-theme identity.** A theme stated in A is recognizable when it returns in A′ — same motif degrees, same home key (`resolve_motif` / `theme_melody_pitch` unchanged). Characters give the theme its *rhythmic/articulative* dress, never its *pitch contour*; saliency routes the theme to the foreground melody, never alters its notes.
8. **THE BYTE-FREEZE ANCHOR (load-bearing).** The `engine_equivalence` golden path — `single_section_default` identity (Bass/Melody/HarmonicFill, goldens 240/114/84/36/79) — **must remain reachable and unchanged.** This is guaranteed by: (a) Ballad's preset being the identity vector (A.10) so the default character reproduces today; (b) every character and saliency scalar defaulting to identity (×1.0 / +0) at the default plan / `prominence = 0`; (c) the `OrchestrationProfile::identity()` path (no pad, no layers) staying the no-op the default Section carries; (d) all new behavior being *additive and gated* behind a non-default character or a non-zero prominence. **Flag:** any build that makes a character/saliency bias non-identity on the default plan, or that touches `realize_step`'s signature, the cadence branch, or the `is_identity()` fast path, **breaks the freeze and is rejected.** The only sanctioned enum edit is adding the `Jubilee` variant + its `parse_character` arm — purely additive, never selected on the identity default.

### C.2 Proposed musical PROPERTY tests (headless, synth-independent)

All run as the existing nets do: build `ImageUnderstanding` / `Section` / `StepPlan` literals directly, drive `generate_chords` / `realize_step` / `realize_rhythm`, never the synth. Each is a *property* (a measurable inequality), not a brittle golden.

**Character properties:**

1. **Energetic > Ballad in rhythmic density (the `example.jpg` cure, encoded).** On the *same* chords/plan, the Jubilee preset yields a strictly **higher mean onset count per step** than Ballad. `density(Jubilee) > density(Ballad) + EPS`. This is the operator's complaint made falsifiable.
2. **Energetic < Ballad in mean articulation fraction (shorter notes).** Same chords: `mean_hold_frac(Jubilee) < mean_hold_frac(Ballad)` (Jubilee's `ARTIC_BIAS` toward 0.55 detaches; Ballad toward 1.10 sings). And March even shorter: `mean_hold_frac(March) ≤ mean_hold_frac(Jubilee)`.
3. **Energetic tempo window exceeds Ballad's.** For an identical bright image, `bpm(Jubilee) > BALLAD_BPM_MAX` is reachable (the cap that caused the failure is lifted for this character); for an identical dark image, Lament `bpm < BALLAD_BPM_MIN` is reachable.
4. **Character changes ≥2 dimensions vs Ballad on a fixed image.** For one fixed `ImageUnderstanding`, the tuple `(bpm, mean_hold_frac, onset-count-distribution, dynamic-level, mode-valence-lean)` differs in **≥2 components** between any two distinct characters — proving a character is a *bundle*, not a single label.
5. **Hymn is more consonant than the image alone would give.** Hymn's `chromaticism_scale < 1` measurably reduces the count of borrowed/secondary-dominant chords vs the Ballad realization of the same (busy/saturated) image.
6. **Mode valence lean works.** A high-valence character on a borderline-hue image picks a major-family mode (Ionian/Lydian/Mixolydian) more often than a low-valence character on the same image.

**Saliency → role properties:**

7. **High-saliency image: melody louder than accompaniment, by more than the baseline gap.** On a strong-subject image (`subject_prominence` high), `mean_velocity(Melody) - mean_velocity(Pad) > baseline_gap + EPS`, where `baseline_gap` is the same difference at `prominence = 0`. The foreground is measurably more foregrounded.
8. **High-saliency: melody wider pitch range and higher mean register than accompaniment.** `pitch_range(Melody) > pitch_range(Pad)` AND `mean_pitch(Melody) > mean_pitch(Pad)`, with the separation strictly larger at high prominence than at zero. (Encodes "wider range, foreground.")
9. **High-saliency: melody more rhythmically active than the background.** `mean_onset_count(Melody) > mean_onset_count(Pad/fill)` on a busy-subject image; the background stays sustained (lower onset count) even when the subject is busy.
10. **Background "larger role" = fuller recessive bed.** A low-`background_energy` image yields a Pad holding *more* tones (`pad_voices` higher) AND at a *lower* mean velocity than the melody — fuller but quieter (the owner's "larger role in the background" with recession intact).
11. **Profile ladder is monotone.** As `subject_prominence` increases across the tier thresholds (B.3), the selected profile moves identity → pad_bed → pad_bed_counter → pad_figured (no regressions; a more salient subject never selects a *less* foregrounded profile).
12. **No inversion invariant (the hard guard).** Across ALL characters and ALL prominence values, for every step: `mean_pitch(Bass) < mean_pitch(any bed/fill voice) < mean_pitch(Melody)` and every emitted note ∈ 24..=108. This is the register-separation invariant (C.1.3) as a sweeping property test.

**Freeze guards (must stay GREEN, no expected change):**

13. **Identity default reproduces the goldens.** The Ballad preset on the `single_section_default` identity path reproduces `engine_equivalence` exactly (240/114/84/36/79). *This is the freeze; it must not move.*
14. **`prominence = 0` reproduces today's role balance.** With no subject, melody/pad velocity and register biases are exactly the current +2 / -3 / G4 — the saliency layer is a pure no-op on a figureless image.
15. **Cadence ring byte-stable.** The 240 ms cadence hold is unchanged under every character and every prominence value (biases are non-cadence only).

### C.3 The invariant I am most worried a build could break

**Register separation under the saliency melody-lift + the energetic character's bright-octave + a bright image's `role_pitch` brightness term, stacking past the band.** Three independent forces all push the melody octave UP — Jubilee's high register, a bright image's `role_pitch` `bright_octaves` lift, and the saliency `+5` foreground lift. If they stack naively, the melody can clamp at the top of 24..=108 and *flatten* (lose its range), or — worse, if a future build also lowers the bed for "contrast" — the bed could rise toward or above a clamped melody and **invert the figure-ground**, which is the exact opposite of what the owner asked for. The mitigation is in the design (lift the melody, never lower the bed; clamp the *sum* of all three lifts, not each independently; saliency widens by raising the foreground only) and is pinned by property test #12, but it is the place a careless additive build is most likely to silently produce a muddy, range-compressed top line — the very "lifeless" symptom we are fixing. **Any build touching the melody register must run #12 and confirm the melody still has measurable range at maximum stacked lift.**

---

*End of S21 musical-craft design. No source, test, or asset modified. Six shipped characters (Ballad default, Hymn, Nocturne, Drone, March, Lament) plus an added energetic/joyful character (Jubilee) for the high-arousal/high-valence corner that `example.jpg` occupies — each defined as a concrete bundle of existing-craft parameters, with Ballad as the identity (freeze-safe) vector. Saliency drives role prominence by reweighting the existing role system (not inventing roles): the most salient subject is foregrounded into the melody (louder, higher, wider, more active) and the least-prominent regions recede into a fuller, quieter, optionally animated background bed — riding on the already-built figured-bed and counter-melody layers and the saliency-conditioned texture SelectTable. Almost everything reuses existing craft; the only new pieces are the per-character scalar bundle threaded as identity-defaulting biases, a per-character tempo window, the one additive `Jubilee` enum variant, and the planner-side prominence vector. The byte-freeze identity path is preserved by construction (Ballad = identity, all biases default to identity, all behavior additive and gated). The invariant most at risk is register separation under stacked melody-lift forces — pinned by a sweeping no-inversion property test.*

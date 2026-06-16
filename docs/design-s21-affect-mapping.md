# Design S21 — Image-Affect → Musical-Character Mapping

**Status:** DESIGN ONLY. This document specifies the affect bridge; it modifies no source or
asset file. The single deliverable is this doc. The `assets/mappings.json` rows it specifies
are handed to the single-writer of that file to merge (see "Shared-file note" at the end).
**Date:** 2026-06-15
**Scope:** the perceptual/affect side of Stage 4 (CHARACTER) of the composition-architecture
roadmap — how an image's affect (valence/arousal) and direct cross-modal correspondences become
musical character and expressive parameters. The music-craft realization of these knobs (voice
leading, chord internals, the realizer multipliers) belongs to the music-theory design; the
engine placement of the composite belongs to the architecture design. This doc owns only the
perceptual mapping.

---

## 0. The failure, located precisely

`example.jpg` is a bright, chaotic, highly-saturated, multi-colored abstract painting (dense
mosaic brushstrokes, full-rainbow palette, no single subject, ~2:1 aspect). It should read as
fast, energetic, and joyful. The current engine produces a slow-to-mid ballad for it — and for
every image. The mechanism, confirmed in code:

1. **Tempo is brightness-only and double-capped.** `brightness_to_tempo_bpm` tops at `120` BPM
   (`assets/mappings.json`), then `composition.rs::plan` re-clamps it to a Ballad window
   `BALLAD_BPM_MIN=56 .. BALLAD_BPM_MAX=96` (`composition.rs:727`, `:851-852`). A maximally
   energetic image cannot exceed 96 BPM. Saturation, colorfulness, complexity, and edge activity
   never touch tempo at all.
2. **Character is constant.** The `character` SelectTable is `{ "default": "ballad", "rules": [] }`
   (`assets/mappings.json:134`). `parse_character` therefore always returns `Character::Ballad`
   (`composition.rs:701`, `:919-933`). Every image is a ballad.
3. **There is no pooled arousal signal.** `avg_saturation` only feeds harmonic complexity;
   `edge_activity` only feeds rhythm/form/section-length. Nothing combines the arousal-bearing
   features into one quantity that co-drives tempo + loudness + density. The `Knob` enum
   (`composition.rs:336-356`) exposes every raw feature but no composite.

**The fix is not "more thresholds."** It is to insert a principled affect bridge: pool the
features into **arousal** and **valence**, drive the macro character knobs from those two axes,
remove the tempo cap, and add the missing energetic/joyful character preset that the
high-arousal/high-valence corner demands.

**Note on the `Character` enum vs the roadmap.** The enum in `composition.rs:177-188` already
carries ten variants (Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt,
Gigue), but the roadmap's musical design (§A.3) ships and parameterizes only **Ballad / Waltz /
March / Lament** — and **none of those is the high-arousal/high-valence preset** `example.jpg`
needs. Identifying and placing that missing preset is part of §2 of this design.

---

## 1. THE BRIDGE — `ImageUnderstanding` fields → VALENCE and AROUSAL

This is the centerpiece. Two continuous scalars on Russell's circumplex
[Russell 1980; Eerola & Vuoskoski 2011]: dimensional, not categorical, so a graded image
produces graded music and ambiguous images land between poles instead of being force-labeled.

All inputs are read from `composition::ImageUnderstanding` (confirmed field names and ranges in
`src/composition.rs:39-88` and produced in `src/pure_analysis.rs::understand_image_pure:639`).
Both outputs are scalars in **[0, 1]** (0.5 = neutral).

### 1.1 Input normalization (everything to 0..1 before pooling)

The energy knobs (`edge_activity`, `texture`, `complexity`, `colorfulness`, `fg_bg_contrast`,
`quadrant_contrast`, the three `*_energy` knobs, `subject_size`) are **already 0..1** as produced
by `understand_image_pure`. The two HSV scalars are **0..100** and must be divided by 100:

| Field | Source range | Normalized term | Note |
|---|---|---|---|
| `avg_saturation` | 0..100 | `s = avg_saturation / 100` | clamp 0..1 |
| `avg_brightness` | 0..100 | `b = avg_brightness / 100` | clamp 0..1 |
| `colorfulness` (== `hue_spread`) | ~0..1 | `c = colorfulness.clamp(0,1)` | already normalized; saturates near 1 |
| `complexity` | 0..1 | `x = complexity` | `clamp(shape_complexity/2,0,1)` |
| `edge_activity` | 0..1 | `e = edge_activity` | `clamp(edge_density/0.05,0,1)` |
| `quadrant_contrast` | 0..1 | `q = quadrant_contrast` | population-stddev of cell values /50 |

`texture` (`clamp(texture_laplacian_var/2000,0,1)`) is available and arousal-positive by the same
Berlyne logic as complexity, but it correlates strongly with `complexity` and `edge_activity`
(all three rise on busy images). To avoid triple-counting the same visual busyness, `texture` is
**held out of the load-bearing composite** and reserved as an optional garnish term (§1.2 note).

### 1.2 THE AROUSAL COMPOSITE (the piece missing today)

Arousal is a weighted sum of the arousal-bearing visual features, saturation dominant, normalized
so the weights sum to 1 and the output lands in [0, 1]:

```
arousal =  0.45 * s          # avg_saturation/100   — DOMINANT arousal driver
         + 0.25 * c          # colorfulness         — color variety / collative variable
         + 0.20 * e          # edge_activity        — visual activity / spatial frequency
         + 0.10 * x          # complexity           — shape/structural busyness
arousal = arousal.clamp(0.0, 1.0)
```

| Term | Weight | Affect dim | Direction | Confidence | Perceptual justification |
|---|---|---|---|---|---|
| `s` avg_saturation/100 | **0.45** | arousal | + (dominant) | **HIGH** | Saturation is *the* dominant arousal driver in the color-emotion literature [Valdez & Mehrabian 1994 (Arousal = −.31·B + .60·S); Wilms & Oberfeld 2018]. Highest weight by design. |
| `c` colorfulness | 0.25 | arousal | + | MEDIUM | Color variety is a Berlyne collative/arousal-potential variable; a chaotic multi-colored image reads energetic [Machajdik & Hanbury 2010; Berlyne 1971]. |
| `e` edge_activity | 0.20 | arousal | + | LOW–MEDIUM | Edge density is a complexity/arousal proxy (spatial frequency / busyness) [Machajdik & Hanbury 2010 by analogy to Berlyne]. No isolated landmark study — tune by ear. |
| `x` complexity | 0.10 | arousal | + (monotone) | MEDIUM | Structural busyness raises arousal monotonically (Berlyne). Lowest weight because it overlaps `e`/`texture`. |

**Why these weights:** saturation gets ~half the mass because it is the only HIGH-confidence
arousal driver; the three MEDIUM/LOW terms split the rest, ordered by how cleanly they isolate
"energy" from "mere detail." The composite is intentionally **monotone in every input** — there is
no inverted-U on the arousal axis (the Wundt inverted-U is a *valence* phenomenon, §1.3).

**Optional garnish (do not load-bear on it):** a small `texture` contribution (e.g. fold a
`0.05*texture` term in and renormalize) is defensible but tune-by-ear; default is to leave it out
to avoid triple-counting busyness. **`quadrant_contrast` and the `*_energy` knobs are NOT pooled
into macro arousal** — `quadrant_contrast` is a balance/form signal (it drives form selection,
not energy), and the energy triplet is a *saliency/role* signal reserved for §4.

**The brightness×saturation interaction (deferred, honest):** Valdez & Mehrabian's full arousal
regression has a mild *negative* brightness term that only turns positive under high saturation.
That interaction is real but second-order and MEDIUM-confidence; folding a `b*s` cross term into
the composite is a later tuning refinement, not part of the load-bearing v1. Omitting it is
conservative (it slightly *under*-reads arousal for bright-but-desaturated images, which is the
safe direction — those should not read as high-energy).

### 1.3 THE VALENCE MAPPING (brightness-led, NOT hue-led)

Valence is **brightness-dominant** [Valdez & Mehrabian 1994: Pleasure = .69·B + .22·S], with
saturation a secondary positive term and two LOW-confidence garnish inputs that nudge but never
own the axis:

```
valence =  0.70 * b                          # avg_brightness/100 — DOMINANT valence driver
         + 0.20 * s                          # avg_saturation/100 — secondary (bright+saturated = most pleasant)
         + 0.10 * fluency                     # figure-ground processing-fluency garnish
valence = valence.clamp(0.0, 1.0)

where fluency = 0.5 + 0.5 * fg_bg_contrast   # a clear figure-on-ground reads as more "resolved"/pleasant
```

| Term | Weight | Affect dim | Direction | Confidence | Perceptual justification |
|---|---|---|---|---|---|
| `b` avg_brightness/100 | **0.70** | valence | + (dominant) | **HIGH** | Brightness dominates pleasure/valence [Valdez & Mehrabian 1994; Wilms & Oberfeld 2018]. This is the load-bearing valence driver. |
| `s` avg_saturation/100 | 0.20 | valence | + | MEDIUM | Valence is highest for bright *and* saturated colors [Wilms & Oberfeld 2018]; saturation's smaller valence coefficient (.22) vs brightness's (.69). |
| `fluency` (from `fg_bg_contrast`) | 0.10 | valence | + | LOW–MEDIUM | Processing fluency → pleasure: a clear figure-ground organization is easier to parse and reads as more pleasant [Reber, Schwarz & Winkielman 2004 — principle mainstream]. Garnish only. |

**THE LOAD-BEARING CAVEAT — valence, not hue, owns major/minor.** Only the major (Ionian) vs
minor (Aeolian) valence contrast is empirically validated [Hevner 1936; Eerola et al. 2013, mode
= top effect size]. `dominant_hue` is **deliberately excluded** from the valence composite: the
warm=happy / cool=sad intuition is the most culturally contingent link in the literature, its sign
reverses with saturation, and Valdez & Mehrabian's own ranking puts blue/green among the *most*
pleasant hues [Wilms & Oberfeld 2018; Valdez & Mehrabian 1994]. **Valence selects mode; hue stays
a colorist garnish** (the existing `hue_to_mode` six-mode spread is kept only for modal *flavor*,
never for the load-bearing major/minor split — see §2.3 and §6).

**Why no inverted-U here either (v1):** Berlyne's Wundt curve predicts valence peaks at *moderate*
complexity and falls at extremes. It is real but the peak location is image-dependent and
MEDIUM-confidence; baking a complexity penalty into valence risks docking `example.jpg` (a
high-complexity image) for being busy, which is exactly the wrong outcome. v1 keeps valence
monotone in brightness; a complexity inverted-U is a later, ear-tuned refinement.

### 1.4 Build-readiness: how the planner reads arousal/valence

The `SelectTable`/`Predicate`/`Knob` machinery (`composition.rs:336-457`) reads scalar knobs out
of `ImageUnderstanding`. There is **no `arousal`/`valence` knob today**. Two build options:

- **(A — RECOMMENDED) Add `arousal` and `valence` as computed fields on `ImageUnderstanding`,
  plus two `Knob` variants (`Arousal`, `Valence`) with one match-arm each in `Knob::read`.** The
  composite is computed once in `understand_image_pure` (the architecture design owns *where*).
  This makes every character/tempo rule a clean single-predicate read of the affect axis, keeps
  the mapping legible, and is the honest realization of "dimensional bridge." It is an additive
  schema change (two struct fields + two enum variants + two `Knob::read` arms), backward-
  compatible (old mappings.json never names the new knobs, so it still parses and behaves).
- **(B — fallback, no Rust change) Approximate the composite inline with multi-predicate AND rules
  over the existing raw knobs.** E.g. "high arousal" ≈ `avg_saturation ge 55 AND (colorfulness ge
  0.5 OR edge_activity ge 0.5)`. This works with zero code change but cannot express the *weighted
  sum* — it can only carve the corner of the V/A plane with axis-aligned boxes, which is coarser
  and harder to tune.

**Recommendation: option A.** It is a tiny code change, it is the correct realization of the
dimensional model, and §2/§3 are written against it. Where a rule below reads `arousal` or
`valence`, that is option A; the option-B desugaring is given once (§2.5) for the no-code path.

---

## 2. AFFECT → THE MACRO MUSICAL ENVELOPE

Arousal drives the "energy" knobs (tempo, loudness, density, articulation, register, texture
density); valence drives the "color" knobs (mode major/minor, consonance). The music-side
directions are the best-established part of the whole chain and combine roughly additively
[Eerola, Friberg & Bresin 2013; Juslin & Laukka 2003]. The music-craft *realization* of each knob
(the actual velocity formula, the rhythm bands, the articulation multiplier) is the music-theory
design's lane — this section specifies the perceptual *direction* and the *value* each affect
level should produce.

### 2.1 TEMPO — remove the cap, drive from arousal

**Two changes.** (a) Raise the `brightness_to_tempo_bpm` ceiling so brightness alone no longer
tops out at mid-tempo. (b) Replace the Ballad-window clamp (`BALLAD_BPM_MIN/MAX = 56/96`) with a
**character-window** clamp whose window is selected by the chosen character — and let the
**arousal composite** position the BPM inside the new, wider musical range.

**New musical tempo range: ~52 BPM (lowest Lament) up to ~168 BPM (highest energetic preset).**
This spans calm-ballad to genuinely-fast without leaving musical territory (168 is brisk allegro,
not a meaningless number).

**The arousal → BPM curve (the de-capped tempo law):**

```
target_bpm = BPM_FLOOR + arousal * (BPM_CEIL - BPM_FLOOR)
           = 52 + arousal * (168 - 52)        # arousal in [0,1]
final_bpm  = target_bpm.clamp(char_bpm_min, char_bpm_max)   # the chosen character's window
```

| Source | Affect dim | Target | Direction | Confidence | Justification |
|---|---|---|---|---|---|
| arousal composite | arousal | tempo (BPM) | faster | **HIGH** | Tempo is the strongest single arousal cue in music [Hevner 1937; Eerola et al. 2013]. The current cap is the proximate cause of the failure. |
| `avg_brightness` (legacy) | — (direct) | tempo base | faster | MEDIUM | Keep brightness as a *secondary* tempo nudge (cross-modal, §3), but arousal — not brightness — is now the primary tempo driver. |

**The mappings.json de-cap** (uncaps the brightness table so it no longer pins 120 as the
ceiling; see §2.6 for exact rows). The hard musical range is owned by the per-character window,
not the brightness table — the brightness table becomes a *secondary* tempo input once arousal is
the primary, so its top anchor is raised to keep it from re-capping the arousal-driven tempo.

### 2.2 The other arousal knobs (loudness, density, articulation, register, voices)

| Source | Affect dim | Target musical param | Direction | Confidence | Justification |
|---|---|---|---|---|---|
| arousal | arousal | dynamic level / loudness (velocity baseline) | louder as arousal ↑ | **HIGH** | [Juslin & Laukka 2003; Eerola et al. 2013] |
| arousal | arousal | rhythmic density / note rate (onsets, figuration) | more/faster onsets as arousal ↑ | **HIGH** | [Juslin & Laukka 2003] |
| arousal | arousal | articulation bias | toward staccato/detached as arousal ↑; legato when calm | MEDIUM–HIGH | [Juslin & Laukka 2003; Eerola et al. 2013]. Rides on top of the §D articulation clamp window. |
| arousal | arousal | register / pitch height (melody baseline) | higher as arousal ↑ | **HIGH** | [Hevner 1937; Eerola et al. 2013] |
| arousal | arousal | texture density / # active voices | denser/more layers as arousal ↑ | MEDIUM | [Webster & Weir 2005]. Realized via the `texture` SelectTable / `OrchestrationProfile` density bias — already present in the schema. |

Each of these is a **plan-supplied scalar** the realizer applies (the roadmap's §A.3 "bundle of
parameters" model). This design specifies the *direction and the affect source*; the music-theory
design owns the exact multiplier and its golden re-derivation.

### 2.3 VALENCE knobs (mode, consonance)

| Source | Affect dim | Target | Direction | Confidence | Justification |
|---|---|---|---|---|---|
| valence | valence | MODE | major/Ionian as valence ↑; minor/Aeolian as valence ↓ | **HIGH** (Western/learned) | Mode is the top effect-size cue and the dominant *valence* carrier [Hevner 1936; Eerola et al. 2013]. **Load-bearing — owned by valence, not hue.** |
| valence | valence | consonance/dissonance | more consonant as valence ↑; more dissonant as valence ↓ | **HIGH** | [Gabrielsson & Lindström 2010]. Realized via the existing saturation→harmonic-complexity / mixture machinery, now *gated by valence* rather than firing per-edge. |

**Mode binding.** Valence selects the *mode family* (major vs minor); `dominant_hue` may still pick
the *specific mode flavor within the family* as a garnish (e.g. which bright mode, which dark
mode), but the major/minor split is valence's alone. Concretely, the cleanest build is a
valence-gated mode choice that overrides the home-mode major/minor decision; the existing
`hue_to_mode` table is retained for color but its output is mapped to the valence-selected family.
**This is a music-theory-design integration point** — this doc specifies that *valence owns the
major/minor bit*; how it composes with `hue_to_mode` is realized by the music-theory writer who
owns `hue_to_mode`. (Flagged in §6.)

### 2.4 THE CHARACTER PRESETS — does the set span the affect space?

A character is a **bundle of parameters** placed on the valence×arousal plane (meter, tempo
window, texture, articulation tendency, dynamic posture). The roadmap §A.3 ships **Ballad / Waltz
/ March / Lament**. Plotting them on the plane:

```
            HIGH AROUSAL
                 │
      (anger/    │   ★ THE GAP ★
      tension)   │   high-arousal + high-valence
        March ───┼───  =  joyful/energetic
       (firm,    │      → NO PRESET TODAY
        4/4,     │
        loud)    │            Waltz
                 │         (lilting 3/4,
 LOW ────────────┼──────────── mid-arousal, +valence) ─── HIGH
 VALENCE         │                                        VALENCE
                 │
       Lament    │        Ballad (DEFAULT)
     (slow, dark,│      (slow, legato, gentle,
      minor,     │       mild-positive valence,
      low-arousal│       LOW arousal)
      −valence)  │
                 │
            LOW AROUSAL
```

**The high-arousal/high-valence quadrant — where `example.jpg` lives — is empty.** Ballad sits at
low-arousal/mild-valence; March is high-arousal but neutral-to-negative valence (firm/martial, not
joyful); Waltz is mid-arousal; Lament is low-arousal/negative-valence. **None of the four shipped
characters is fast AND bright AND joyful.** This is *the* structural cause of "no energy, no joy"
for `example.jpg` — even after the tempo cap is removed, with only these four presets the planner
has nowhere joyful-and-fast to send the image.

**RECOMMENDATION: add ONE energetic/joyful character preset** to fill the empty quadrant. The
`Character` enum already has unused variants that fit — **`Scherzo`** (lit. "joke": fast, light,
playful, major-leaning) or **`Gigue`** (fast, bright, dancing, compound feel) are both apt
existing enum members, so this needs **no enum edit** — only a mappings.json `character` rule and
the music-theory design's parameter bundle for it. I recommend **`Scherzo`** as the high-arousal/
high-valence preset (it reads "joyful/playful/energetic" most directly and is the standard label
for that affect in the concert-music vocabulary; `Lilt` is a softer mid-arousal alternative).

**Proposed preset placement on the plane (the perceptual contract; the music-theory design fills
the exact tempo/texture/articulation numbers):**

| Character | V/A corner | Arousal band | Valence band | Tempo window (BPM) | Mode lean | Articulation | Perceptual reason |
|---|---|---|---|---|---|---|---|
| **Scherzo** *(NEW — fill the gap)* | high-A / high-V | high (≥~0.6) | high (≥~0.55) | **120–168** | major | detached/light | The joyful-energetic preset. `example.jpg`'s home. |
| Waltz | mid-A / +V | mid | mid–high | 96–144 | major-lean | portato/legato mix | Lilting, characterful, positive but not frantic. |
| March | high-A / neutral–low-V | high | low–mid | 96–120 | either | marcato/detached | High energy *without* joy — firm/martial; distinguishes anger-corner energy from joy. |
| Ballad *(DEFAULT)* | low-A / mild-V | low | mild-+ | 56–88 | either | legato | Safe pleasant; the calm default. |
| Lament | low-A / −V | low | low | 52–66 | minor | legato, weighted | Sad corner; slow + minor + (musically) *soft*, not loud (§6 fear/sadness caveat). |

This is a **five-preset set** (the four roadmap presets + Scherzo) that now spans all four
quadrants of the plane. Confidence that the *set spans the space*: HIGH (with Scherzo added);
without it, the high-V/high-A corner is unreachable.

### 2.5 The `character` SelectTable rules (affect → preset)

The `character` table is selected by affect, first-match-wins, with `ballad` as the safe default
when no affect rule fires. Reading the affect knobs directly (option A):

| Order | Rule (predicates AND'd) | Picks | V/A corner | Confidence |
|---|---|---|---|---|
| 1 | `arousal ge 0.60` AND `valence ge 0.55` | **scherzo** | high-A / high-V | HIGH |
| 2 | `arousal ge 0.60` AND `valence lt 0.45` | **march** | high-A / low-V | MEDIUM |
| 3 | `arousal le 0.35` AND `valence lt 0.40` | **lament** | low-A / low-V | MEDIUM |
| 4 | `arousal in_range 0.40..0.70` AND `valence ge 0.55` | **waltz** | mid-A / high-V | MEDIUM |
| (default) | — | **ballad** | low-A / mild-V | safe fallback |

Thresholds are the **starting calibration, tune by ear** (the owner's ear is the gate — see §6).
The boundaries are intentionally non-exhaustive with `ballad` as the catch-all so any
unclassified image degrades to the safe pleasant default rather than to a wrong strong character.

**Option-B desugaring (no-code path)** of rule 1, for reference, reading raw knobs instead of the
composite: `avg_saturation ge 55` AND `avg_brightness ge 55` AND `(colorfulness ge 0.45 OR
edge_activity ge 0.45)`. This is coarser (axis-aligned, no weighting) and is the fallback only if
the `Arousal`/`Valence` knobs are not added.

### 2.6 The mappings.json rows to merge (tempo de-cap + character table)

These are content rows on the existing schema — **not a schema change** (option A's two `Knob`
variants are a separate, small Rust change owned by the architecture design; the rows below assume
those knobs exist). Backward-compatible: old mappings.json omits these and still parses.

**(a) De-cap `brightness_to_tempo_bpm`** (raise the top anchor so brightness is no longer a hard
120 ceiling once arousal is the primary tempo driver):

```jsonc
"brightness_to_tempo_bpm": {
  "0-30":  72,     // was 60  — floor lifted slightly so dark-but-energetic isn't dirge-slow
  "31-70": 108,    // was 90
  "71-100": 150    // was 120 — CAP REMOVED; bright images can reach fast territory
}
```

**(b) The affect-driven `character` SelectTable** (replaces the empty-rules default-pinned table):

```jsonc
"character": {
  "default": "ballad",
  "rules": [
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0},
                {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "scherzo" },
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0},
                {"knob":"valence","op":"lt","lo":0.45,"hi":0.0} ], "pick": "march" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.35,"hi":0.0},
                {"knob":"valence","op":"lt","lo":0.40,"hi":0.0} ], "pick": "lament" },
    { "when": [ {"knob":"arousal","op":"in_range","lo":0.40,"hi":0.70},
                {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "waltz" }
  ]
}
```

**(c) Per-character tempo windows.** The Ballad-window constants `BALLAD_BPM_MIN/MAX` in
`composition.rs:851-852` must become a **per-character window** keyed by the selected `Character`.
The window table (the music-theory design owns the exact numbers; these are the perceptual
contract from §2.4):

```jsonc
// proposed new "character_tempo_window" block (architecture/music-theory design wires the lookup;
// this doc supplies the windows that make each preset audibly its affect)
"character_tempo_window": {
  "scherzo": { "min": 120, "max": 168 },
  "march":   { "min": 96,  "max": 120 },
  "waltz":   { "min": 96,  "max": 144 },
  "ballad":  { "min": 56,  "max": 88  },
  "lament":  { "min": 52,  "max": 66  }
}
```

This replaces the single hardcoded 56..96 clamp with a window the chosen character selects —
**this is the de-cap of the Ballad clamp.** (Whether this rides as a JSON block or as character
constants is an architecture/music-theory build decision; the *windows* are the perceptual ask.)

---

## 3. THE SECONDARY CROSS-MODAL PATH (per-section / per-voice)

Direct visual↔auditory correspondences that map a feature to a sound parameter **without routing
through emotion**. These ride at the **per-bar / per-voice** level (where they are cheaper and
more defensible than an affect detour), distinct from the §1–§2 macro affect envelope which sets
the piece's overall character. The affect bridge owns *macro* character; these own *local* detail.

| Source field | Auditory target | Direction | Confidence | Where it applies | Justification |
|---|---|---|---|---|---|
| per-bar `avg_brightness` | pitch height | brighter → higher pitch | **HIGH** | per-bar melody-note selection | Brightness↔pitch is a low-level sensory correspondence [Marks 1987; McCormick et al. 2018]. |
| `subject_size` (`subject.area_frac`) | pitch | bigger subject → **lower** pitch | **HIGH** | melody/bass register seed | Large objects vibrate lower [Gallace & Spence 2006; Spence 2011]. |
| `vertical_emphasis` / `mass_centroid.y` | register | higher in frame → higher pitch | **HIGH** | per-section/voice register band | Elevation↔pitch [Walker et al. 2010; McCormick et al. 2018]. |
| `edge_activity` / `subject_energy` (per-bar) | tempo / event rate | busier → faster onsets | MEDIUM–HIGH | per-bar rhythmic density (within the section tempo) | Visual motion↔temporal rate [Spence 2011; Eitan & Granot 2006]. |
| `edge_activity` (angularity proxy) | timbral sharpness / dissonance | angular/jagged → sharp/dissonant; rounded → smooth | **HIGH** (~95% cross-cultural) | per-bar non-chord-tone / brightness of timbre | Bouba/kiki [Köhler 1929; Ćwiek et al. 2022]. |

**Boundary discipline:** these correspondences are realized in the per-step realizer (music-theory
/ architecture lane). This design specifies the perceptual direction; it does not author the
realizer. Note `edge_activity` does double duty — it contributes to the *macro* arousal composite
(§1.2) AND rides as a *local* per-bar event-rate / angularity cue here; that is intentional and
not double-counting because the two operate at different scales (one sets the section tempo
window, the other modulates within it).

---

## 4. SALIENCY → ROLE (the owner's melody-from-subject intuition)

The owner's intuition — **the most salient/prevalent subject drives the MELODY; the least-prominent
regions drive the BACKGROUND/accompaniment** — is well-motivated: visual saliency is real and
computable [Itti et al. 1998], figure-ground is a shared organization across vision and audition
[Bregman 1990], and the melody is the auditory "figure" with high-voice superiority [Trainor et
al. 2014]. It is a **principled design heuristic**, not a validated perceptual law — implement it,
document it as such (MEDIUM confidence).

`understand_image_pure` already produces the saliency signals (`pure_analysis.rs:719-735`):
`subject_energy` (edge energy of the salient subject cell), `foreground_energy` (edge-mid band),
`background_energy` (corner band), `subject_size` (`subj.area_frac`), `fg_bg_contrast`. The
perceptual → role-prominence mapping:

| Saliency signal | Musical role prominence | Direction | Confidence | Justification |
|---|---|---|---|---|
| `subject_energy` (the salient subject's energy) | **MELODY prominence/activity** | higher subject energy → more active, more foregrounded melody | MEDIUM | The salient subject is the auditory figure (melody) [Itti 1998; Trainor 2014]. |
| `fg_bg_contrast` | **degree of figure/ground stratification** | high contrast → strong melody-vs-accompaniment separation; low contrast → homogeneous texture (no clear lead) | MEDIUM | A real subject-on-ground → a real melody-on-accompaniment; a flat field → no figure, so no forced lead [Bregman 1990 figure-ground]. |
| `background_energy` (corner/border band) | **ACCOMPANIMENT / pad activity** | higher background energy → busier accompaniment bed | MEDIUM (heuristic) | Least-prominent regions → the harmonic bed [Bregman 1990; no direct validating study]. |
| `subject_size` | melody register (cross-modal, §3) | bigger subject → lower melody register | HIGH (size↔pitch) | [Gallace & Spence 2006]. |
| (melody placement, always) | melody → the **high voice** | melody seated toward the top of the texture | MEDIUM–HIGH | High-voice superiority [Trainor et al. 2014]. |

**How affect interacts with saliency:** **affect sets the *energy of the whole texture*; saliency
sets *how that energy is distributed across roles*.** High arousal makes the *entire* piece more
active (faster, denser, louder — §2); `fg_bg_contrast` then decides whether that activity is
*concentrated in a clear melodic figure* (high contrast → a prominent lead over a bed) or *spread
evenly* (low contrast → a homogeneous, non-hierarchical texture). For `example.jpg` — high arousal
but **low** `fg_bg_contrast` (no single subject in a mosaic field) — the prediction is: very
energetic overall, but **no strongly foregrounded single melody** — an even, busy, joyful texture
rather than a tune-over-accompaniment. That matches the painting: it has no subject, so the music
should not invent a soloist. (This is exactly what the existing `texture` SelectTable already
encodes — `pad_bed_counter`/`pad_figured` fire only when `fg_bg_contrast` is high enough, falling
back to the homogeneous `pad_bed` otherwise; the affect layer should respect that gate.)

**Boundary:** the musical-craft realization of roles (voice leading, which `OrchestralRole` plays
the motif, the bed voicing) is the music-theory design's lane. This doc provides only the
perceptual mapping: *which saliency signal maps to which role-prominence, and how affect scales
the whole.*

---

## 5. THE PURE-RUST vs ML LINE (per desired effect, honest)

| Desired effect | Pure-Rust reachable? | How / why |
|---|---|---|
| **Energy / high arousal** | **YES (now)** | The §1.2 arousal composite (saturation+colorfulness+edge_activity+complexity) → §2 uncapped tempo + loudness + density. This is the direct fix for the current failure. |
| **Fast-paced** | **YES (now)** | Same composite → tempo with the cap removed (§2.1, §2.6). |
| **Joy — the ACOUSTIC signature** | **PARTIAL but reachable (now)** | brightness → valence → major + consonant + (with high arousal) fast/bright/detached = the *acoustic* signature of happiness, via the new Scherzo preset. Reachable from pure features. |
| **Joy — SEMANTIC** (knowing the scene depicts something joyful: a smile, a celebration) | **NO — needs ML** | Requires object/scene recognition. Pure low-level features read texture/color/structure, not subject matter. Opt-in `--features semantic` tier, LATE (roadmap Stage 10), default OFF. **Do not design it in.** |
| **Reliable warm=happy / cool=sad** | **NO / unreliable** | hue→valence is weak, sign-unstable, culturally contingent (§1.3). Demoted to garnish; not a control axis. |

**Net:** the owner's three desired effects — **energy, joy, fast-paced** — are **largely reachable
from pure-Rust features**, because they correspond to arousal (saturation/colorfulness/complexity/
edge-driven) and the acoustic signature of positive valence (brightness-driven major/consonant/
fast). The "always a ballad" failure is **not a limit of pure Rust** — it is a missing arousal
composite, a tempo cap, and a missing joyful character preset, all fixable in the pure-Rust
default. Semantic affect (recognizing *why* an image is joyful) stays an opt-in later tier.

---

## 6. RISKS / CAVEATS (and what is tuned by ear vs from a landmark study)

1. **LOAD-BEARING: valence owns mode, not hue.** Only major/minor is empirically validated [Hevner
   1936; Eerola 2013]. `dominant_hue` is excluded from the valence composite and from the
   major/minor choice; it survives only as a colorist garnish (modal *flavor* within the
   valence-selected family). Getting this wrong reintroduces the contested warm=happy rule as a
   control axis. **Integration point with the music-theory design** (who owns `hue_to_mode`): this
   doc asserts valence owns the major/minor bit; the composition with `hue_to_mode`'s flavor is
   theirs to realize.
2. **MUSICAL fear/sadness = SOFT, not loud.** In music (unlike vocal startle), fear/sadness =
   fast/slow + minor + *low/soft* loudness, which distinguishes them from anger (fast + minor +
   LOUD) [Cespedes-Guevara & Eerola 2018]. The arousal→loudness rule (§2.2) is monotone, so a
   naïve high-arousal-minor image would read as anger by default. **Lament must override loudness
   to soft** despite its place on the plane — the Lament preset's dynamic posture is soft *by
   character bundle*, not by the generic arousal→loudness law. Flagged so the music-theory design
   does not let arousal→loud win for the sad/fearful corner.
3. **Garnish demotions (LOW confidence, ear-tuned):** `dominant_hue`→valence (excluded); the full
   six-mode church-mode brightness ordering (kept as flavor only — only major/minor is validated);
   `fg_bg_contrast`→valence fluency term (small 0.10 weight, garnish). The load-bearing engine is
   built only on the HIGH-confidence links (saturation→arousal, brightness→valence, arousal→tempo/
   loudness, valence→mode).
4. **Tuned by ear, not from a landmark study:** the **arousal-composite weights** (0.45/0.25/0.20/
   0.10), the **valence weights** (0.70/0.20/0.10), the **character thresholds** (§2.5), and the
   **per-character BPM windows** (§2.6c) are a principled *starting* calibration. The owner's ear
   is the gate — these are the first knobs to turn if `example.jpg` reads too fast/slow or the
   character flips at the wrong boundary. The *directions and the relative ordering* are from the
   literature (HIGH/MEDIUM/LOW as tagged); the *exact numbers* are seed values.
5. **`edge_activity` calibration sensitivity.** `edge_activity = clamp(edge_density/0.05, 0, 1)`
   saturates at edge_density ≥ 0.05; a very busy painting like `example.jpg` likely pins it at 1.0.
   That is fine for arousal (it *should* read maximal busyness), but means edge_activity loses
   discrimination among already-busy images — saturation/colorfulness carry the arousal gradient
   there. Acceptable; noted so it is not mistaken for a bug.
6. **Texture held out of the composite** to avoid triple-counting busyness with complexity/edge
   (§1.1). If high-arousal images still under-read after tuning saturation/colorfulness, folding a
   small texture term back in is the documented next lever.
7. **Build dependency on option A.** The clean version (§1.4) needs two `ImageUnderstanding`
   fields + two `Knob` variants. If the operator declines the code change, the option-B desugaring
   (§2.5) is the no-code fallback at coarser fidelity. Recommend option A.

---

## 7. VALIDATION PREDICTION — what `example.jpg` SHOULD produce

`example.jpg` is a bright, highly-saturated, multi-colored, dense-brushstroke abstract painting
with no single subject (~2:1 aspect). Expected feature reads (qualitative, to be confirmed by
running the analyzer): `avg_saturation` high (~75–90 → s ≈ 0.80), `colorfulness`/`hue_spread` high
(full rainbow → c ≈ 0.8–1.0), `edge_activity` saturated (dense strokes → e ≈ 1.0),
`complexity` high (x ≈ 0.6–0.9), `avg_brightness` moderate-high (saturated reds/oranges/blues →
b ≈ 0.55–0.70), `fg_bg_contrast` LOW (no subject → ~0.1–0.2).

**Predicted affect:**
```
arousal ≈ 0.45*0.80 + 0.25*0.90 + 0.20*1.0 + 0.10*0.75
        ≈ 0.36 + 0.225 + 0.20 + 0.075  ≈  0.86   (HIGH)
valence ≈ 0.70*0.62 + 0.20*0.80 + 0.10*(0.5+0.5*0.15)
        ≈ 0.434 + 0.16 + 0.058  ≈  0.65   (HIGH-ish)
```

**Predicted musical output under this mapping:**
- **Character:** `arousal 0.86 ≥ 0.60` AND `valence 0.65 ≥ 0.55` → **Scherzo** (the joyful-energetic
  preset). The always-ballad default is broken for this image.
- **Tempo:** `target_bpm = 52 + 0.86*(168-52) ≈ 52 + 99.8 ≈ 152 BPM`, clamped to the Scherzo
  window 120..168 → **~152 BPM** (fast). Versus today's ≤96 BPM ballad cap — a ~1.6× speedup into
  genuinely-fast territory.
- **Mode:** valence 0.65 (high) → **major / Ionian**, consonant. Joyful, not somber.
- **Dynamics:** high arousal → **loud / strong** velocity baseline.
- **Density / articulation:** high arousal → **dense onsets, detached/staccato-leaning**
  articulation (energetic, not sustained-ballad legato).
- **Register:** high arousal → **higher** melody baseline.
- **Role distribution:** LOW `fg_bg_contrast` → **homogeneous busy texture** (the `pad_bed`
  fallback), **no forced single melodic soloist** — an even, energetic, joyful field, matching a
  subjectless mosaic painting.

**One-line acceptance test for the build:** *`example.jpg` → Scherzo, ~150 BPM, major, loud,
dense, detached, homogeneous texture.* If the owner's ear hears energy + joy + fast pace, the
mapping works; if it reads too frantic or not joyful enough, the first knobs to turn are the
arousal-composite weights, the Scherzo tempo window (§2.6c), and the §2.5 thresholds (§6 item 4).

---

## Shared-file note (assets/mappings.json single-writer)

`assets/mappings.json` is a single-writer file shared with the music-theory design (which owns its
harmonic/progression/extension tables). This design does **NOT** commit it. The exact rows to
merge are §2.6 (a) the de-capped `brightness_to_tempo_bpm`, (b) the affect-driven `character`
SelectTable, and (c) the `character_tempo_window` block; the §1.4 option-A code change (two
`ImageUnderstanding` fields + two `Knob` variants `Arousal`/`Valence` + two `Knob::read` arms +
the composite computed in `understand_image_pure`) is a separate, small change owned by the
architecture design. One writer integrates the JSON rows; the affect rows ride on the existing
`SelectTable`/`brightness_to_tempo_bpm` schema (a content change, backward-compatible — old
mappings.json still parses). The merged result goes through the quality gate (mappings.json
backward-compat + value-range check) like any mappings.json change.

`// TODO(s21-affect-mapping): merge the §2.6 affect/character/tempo rows into mappings.json
(coordinate with the music-theory single-writer); land the §1.4 option-A Arousal/Valence
Knob + ImageUnderstanding composite with the architecture design.`

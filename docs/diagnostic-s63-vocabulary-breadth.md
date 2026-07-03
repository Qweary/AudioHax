# Diagnostic S63 — Generative Vocabulary Breadth ("lacking seeds on all fronts") — READ-ONLY

**Owner:** Music Theory Specialist · **Scope:** `src/composition.rs` (the selector/seam layer) + `src/chord_engine.rs` (motif/rhythm-cell/progression vocabularies) + `assets/mappings.json` (the authored catalogues). **No fix this session.** `src/engine.rs` is FROZEN and is not touched.

**Operator report (after the S62 un-gate landed):** the *notes* now differ across images (good), BUT **"the rhythms sound repetitive"** and **"the overall feel of the accompaniment and melody feels like it's the same thing with variations."** Operator theory, verbatim: *"our library of unique seeds to start with and then vary are lacking on all fronts."*

**Verdict (one line):** The theory is **half right, and the half it's right about is the half that matters most for what the operator hears.** "Lacking on **all** fronts" is **overstated** — the *pitch motif* (S62-fixed: 5/8 contours, 6/6 distinct subjects), *cross-piece rhythm* (4/4 motto cells fired), and *harmonic progressions* (2–3 distinct per piece) are **adequate to healthy**. But three of the four elements the operator is actually hearing are **genuinely collapsed — not because the libraries are too small, but because their SELECTORS are starved**, so a rich authored catalogue funnels onto 1–2 entries on the real-photo cluster, and two of them **never develop within a single piece.** The accompaniment-figuration library (9 patterns) fires on **1 of 6 images**; the rhythmic motto is **one fixed cell for an entire piece**; the form library (6) collapses to **rounded_binary on 5 of 6 images**; and the bass-pattern library (5) is **100% unused** (sustained on every piece). The common root cause is a **selector-starvation pattern identical to the one S62 just fixed for the pitch gate** — the material exists, the routing doesn't reach it.

Empirical harness: `tests/vocab_diag_s63.rs` (`cargo test --release vocab_diag_s63 -- --nocapture`), run on the real `CompositionPlanner::plan` path over the six bundled images. All numbers below are measured, not asserted.

---

## 1. The full vocabulary census (SIZE / corpus-utilization / within-piece)

| # | Vocabulary | Where | SIZE (authored) | CORPUS UTIL (distinct fired /6 imgs) | WITHIN-PIECE (develops per piece?) | Health |
|---|---|---|---|---|---|---|
| 1 | **Accompaniment figuration** | `figuration_catalogue` + `texture` selector | **9** figures (alberti, broken_up, broken_wave, arp_waltz, block_comp_24, oom_pah, oom_pah_pah, stride, block) | **≈1** — only `broken_chord_up` ever resolves; **5/6 images get figuration = NONE** (static block pad) | **1/6** — S45 depart/return arc fires only on `example` (broken>block>block); the other 5 loop one sustained bed | ❌ **COLLAPSED** |
| 2 | **Rhythm motto / cell** (the gait) | `rhythm_cells()` (8×4) + `pick_piece_cell` | **4** cells / archetype (32 contour-gaits) | **4/4** cells fired (cells 0,1,2,3 all used; cross-piece healthy) | **1 cell fixed for the WHOLE piece** — every section walks the identical gait; no rhythmic contrast Statement↔Contrast↔Return | ⚠️ **within-piece FLAT** |
| 3 | **Form / section template** | `form_catalogue` + `form` selector | **6** forms | **2/6** — `theme_and_variations` (1 img) + `rounded_binary` (**5 imgs**) | roles vary inside a form (Statement/Contrast/Return) — but 5/6 pieces share the *identical* A-B-A′ skeleton | ⚠️ **COLLAPSED cross-piece** |
| 4 | **Bass-line pattern** | `bass_pattern_catalogue` + `pad_walking`/`pad_pedal` | **5** (sustained, walking, walking_q, pedal, pedal_dom) | **1/5** — `sustained` default on all 6; walking/pedal **never fire** | none — sustained root every step, every section | ❌ **100% DEAD** (4/5 unreachable) |
| 5 | **Harmonic progression** | `progression_families` + `pick_progression` | **14** (warm 7 / cool 5 / neutral 2) | family **2/3** (warm+cool; neutral never fires); within a fired family, several distinct | **2–3 distinct progressions per piece** (random draw per section) | ✅ **HEALTHY** (but random/non-reproducible) |
| 6 | **Motif archetype (pitch)** | `MotifArchetype::contour()` | **8** contours | **5/8** fired; **6/6 distinct (archetype,motto) identities** | theme recurs on Statement/Return (same subject, by design) | ✅ **HEALTHY** (S62-fixed) |
| 7 | **Character** (tempo/affect) | `character` selector + `character_tempo` | 10 enum / **6 reachable** (ballad,hymn,lament,march,nocturne,scherzo; drone/waltz/lilt/gigue unreachable) | **4/6 reachable** fired (Hymn,Lament,March,Scherzo) | one character per piece | ✅ adequate |
| 8 | **Prominence** (who's foreground) | `prominence_catalogue` + selector | **5** | **3/5** (melody_lead_gentle/strong, subject_melody) | one profile per piece | ✅ adequate |
| 9 | **Key scheme** (modulation) | `key_scheme_catalogue` | **9** | excursion fires on **2/6** (Img1,Img2 modulate; 4/6 static home) | offsets move per section when an excursion scheme is picked | ⚠️ thin but not the complaint |
| 10 | **Prog. selection determinism** | `pick_progression` `thread_rng` | — | — | — | ⚠️ **non-reproducible** (random, not seeded) |

### Per-image evidence (real planner)

| image | form | character | mode/fam | texture | figuration arc | motto | theme (arch / notes / Σdur) |
|---|---|---|---|---|---|---|---|
| AudioHaxImg1 | rounded_binary | Lament | Dorian/cool | pad_bed_counter | **none>none>none** | cell 1 fixed | LeapStep / 3 / 5 |
| AudioHaxImg2 | rounded_binary | Hymn | Lydian/warm | pad_bed_counter | **none>none>none** | cell 0 fixed | Arch / 4 / 5 |
| AudioHaxImg3 | rounded_binary | Scherzo | Mixolydian/warm | pad_bed_counter | **none>none>none** | cell 3 fixed | Arch / 4 / 5 |
| example | theme_and_variations | March | Dorian/cool | pad_broken_up | **broken_up>block>block** | cell 2 fixed | Ascent / 5 / 8 |
| Lena | rounded_binary | Scherzo | Ionian/warm | pad_bed | **none>none>none** | cell 0 fixed | InvertedArch / 4 / 5 |
| magicstudio | rounded_binary | March | Mixolydian/warm | pad_bed | **none>none>none** | cell 3 fixed | Descent / 5 / 8 |

**Read the `figuration arc` and `motto` columns together — they are the whole diagnosis of the two operator complaints:**

- *"the accompaniment feels like the same thing with variations"* → 5 of 6 pieces land on `pad_bed`/`pad_bed_counter`, the **only two `pad_*` profiles with no `figuration` handle**, so their accompaniment is a **sustained block pad with zero animation and zero within-piece development**. It literally *is* the same thing on 5/6 pieces. The 9-figure library and the S45 depart/return arc are exercised on **one** image (`example`).
- *"the rhythms sound repetitive"* → the per-piece motto is a **single cell held for the entire piece**. Cross-piece the four cells spread perfectly (0/1/2/3 all fired), so images differ from each other — but *inside* one piece the gait never changes across Statement→Contrast→Return, so a ~24–32-bar piece walks with exactly one rhythmic figure start to finish.

---

## 2. Why the collapse — the shared root cause (selector starvation)

Offenders #1, #3, #4 fail for the **same structural reason**, and it is the *identical pattern S62 just fixed for the pitch motif gate*: a **rich authored library sits behind a selector whose thresholds the real-photo cluster never clears**, so everything falls through to the one or two default/low-gate entries.

- **Figuration (#1):** the `texture` SelectTable routes calm/low-arousal images (the majority of real photos) to `pad_bed`/`pad_bed_counter` — the *un-figured* profiles. The 8 figured profiles (`pad_figured`, `pad_broken_*`, `pad_arp_waltz`, `pad_block_comp`, `pad_oom_pah*`, `pad_stride`) require `arousal≥0.60`, `colorfulness≥0.40/0.50`, `valence≥0.65`, etc. — bands natural photos rarely hit. Result: **10 of 13 texture profiles are effectively unreachable on the corpus**, and the figuration + bass libraries that hang off them starve with them.
- **Form (#3):** the `form` selector's five non-default rules gate on `complexity≥0.66 ∧ edge≥0.6`, `quadrant_contrast≥0.6`, `aspect_ratio≥1.6`, `vertical_emphasis≥0.6`, `edge≥0.7 ∧ value_key≥0.6` — every one a high threshold; 5/6 images clear none and fall to `rounded_binary`.
- **Bass (#4):** `walking`/`pedal` are reachable ONLY via `pad_walking`/`pad_pedal`, which the texture selector never picks → the entire bass-pattern library is dead weight.

The motto (#2) is the exception: its *selector* is healthy (un-gated in S53, spreads 4/4). Its failure is a **missing mechanism**, not a starved selector — there is **no within-piece motto development** analogous to the S45 figuration arc. One cell is chosen per piece and cloned onto every section unchanged.

**So the operator's "lacking seeds" is precise on the SYMPTOM (the ear hears one accompaniment, one gait) but slightly off on the CAUSE (for #1/#3/#4 the seeds EXIST — the router won't reach them; only #2 needs a new develop-within-piece mechanism, and arguably the figured profiles could use a few more curated figures once the routing is opened).**

---

## 3. Ranked top offenders (weighted by perceptual impact: rhythm + accompaniment first)

### #1 — [DOMINANT] Accompaniment figuration is dead on 5/6 pieces
**Numbers:** SIZE 9 figures / util ≈1 fired / within-piece 1 of 6 images. 5/6 images = static block pad, no figure, no development.
**Why it bites hardest:** this is the exact thing the operator names ("the accompaniment feels like the same thing"). A sustained pad under a melody is the *most* generic accompaniment texture there is, and 5/6 pieces have precisely that.
**Smallest high-leverage expansion (design only):** **decorrelate/re-tune the `texture` selector so the calm-photo cluster reaches the figured profiles** — this is a *routing* fix mirroring S62's un-gate, NOT new material. Two sub-moves: (a) give `pad_bed`/`pad_bed_counter` a gentle **default figuration handle** (e.g. `arp_waltz` or a soft broken cell) so "no figure" is never the outcome; (b) lower/redistribute the figured-profile gates (or add a low-arousal broken-pad route) so ordinary photos spread across ≥3–4 figures instead of 0. **Risk to hold:** keep the selector choosing among the **9 authored figures** only — never synthesize an onset table from a hash; the curated `figuration_catalogue` is the safety rail. If more breadth is wanted *after* routing is opened, add hand-authored curated figures, don't randomize.

### #2 — [HIGH] Rhythm motto never develops within a piece
**Numbers:** SIZE 4 cells/arch / util 4/4 (cross-piece healthy) / within-piece **1 cell fixed for the entire piece**.
**Why it bites:** this is the operator's "rhythms sound repetitive." Cross-piece rhythm is already fine; the collapse is entirely on the *within-piece* axis — a single gait looped for ~24–32 bars.
**Smallest high-leverage expansion:** add a **within-piece motto arc, the rhythmic analogue of the S45 figuration arc** — anchor roles (Statement/Return/Coda) hold the piece's base cell; departure roles (Contrast/Development) take a **contrasting cell from the SAME archetype's 4-cell vocabulary**, so the rhythm departs-and-returns (broad A → busy B → broad A′). Lives entirely in `composition.rs` (the motto rides `OrchestrationProfile`, which the planner sets per section — same seam S45 used; `engine.rs` stays FROZEN). **Risk:** none musical — all 4 cells are authored, voice-leading-neutral gait templates; this reuses existing material, adds no new randomness.

### #3 — [HIGH] Form collapses to rounded_binary on 5/6 pieces
**Numbers:** SIZE 6 forms / util 2/6 / 5 of 6 pieces share the identical Statement-Contrast-Return skeleton.
**Why it bites:** the form is the scaffold on which melody + accompaniment interlock and develop; identical scaffolds are a large part of "the overall feel is the same thing with variations."
**Smallest high-leverage expansion:** **re-tune the `form` selector thresholds** (or add lower-threshold routes) so the calm-photo cluster spreads across ≥3–4 of the 6 authored forms — routing only; the forms already exist. **Risk:** routing only; keep the six authored forms as the closed set.

### #4 — [MEDIUM] Bass-pattern library 100% unused
**Numbers:** SIZE 5 / util 1/5 (sustained only) / walking+pedal never fire.
**Why it bites:** a sustained bass root on every piece is uniform low-end; contributes to accompaniment sameness, though below figuration in salience.
**Smallest high-leverage expansion:** either add `texture` routes that select `pad_walking`/`pad_pedal` for appropriate images, or **attach a bass pattern independent of the figured-pad gate** (so bass motion isn't hostage to the same starved selector). Routing only. **Risk:** none new.

### #5 — [FLAG, not an offender] Progression is the ONE healthy-but-RANDOM axis
Progressions vary well (2–3 distinct/piece, 14-strong library) — the operator is *not* short of harmonic seeds. But they're drawn with `thread_rng` (non-reproducible; family 2/3 since no image maps to a neutral mode). This is the **structure-vs-randomness** tension the operator flagged, inverted: here variety is *un-curated randomness*. If the other axes get decorrelated *deterministically*, consider seeding this one too so the whole piece is reproducible — but it is **not** a "lacking seeds" problem.

**Explicitly NOT offenders (the "not all fronts" half):** the **pitch motif** (5/8 contours, 6/6 distinct subjects, lengths floored 5–8 — S62 fixed it), **cross-piece rhythm** (4/4 cells), **character** (4/6 reachable fired), **prominence** (3/5). These are adequate; expanding them would be low-leverage.

---

## 4. One-line summary for the build lead

The perceived sameness is **real and localized**: **accompaniment figuration (5/6 pieces = static pad, library fires 1/6)** and **within-piece rhythm (one motto cell per whole piece)** are the two dominant offenders — exactly the two the operator named — followed by **form collapse (5/6 = rounded_binary)** and a **100%-dead bass library (1/5)**. For #1/#3/#4 the fix is **selector re-routing (the material already exists, the S62-style un-gate pattern)**; for #2 it's a **new within-piece develop mechanism (the rhythmic twin of the S45 figuration arc)**. Hold the structure-vs-randomness rail throughout: expand by opening routes to **curated** libraries and adding **authored** cells, never by synthesizing material from a hash. `engine.rs` stays FROZEN — every proposed change lives in `composition.rs` / `chord_engine.rs` / `mappings.json`.

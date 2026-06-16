# Input S26 K2a — Per-region excursion direction (Music Theory lane)

**Author role:** Music Theory Specialist — K2a INPUT / VALIDATION DOC. **DOC ONLY:** this file
modifies no source, test, or asset and owns no `.rs` file. It validates (and where needed corrects)
the per-region direction rule the Implementer transcribes into `region_excursion_offset` +
`resolve_key_scheme` for the K2a slice, at a professional-ear standard.
**Date:** 2026-06-16. **HEAD:** `9cd9681`.
**Builds on (do not restate, kept verbatim where cited):**
`docs/input-s26-multiexcursion-harmony.md` (the menu `{+7,+5,+3,−3}`, the `relative_offset`
mode-family rule, the direction-from-affect table §3.1, the smoothness-ranked distinctness advance
§3.2, the direction guard §3.3); `docs/input-s25-k1-keyplan-harmony.md` (the trained-ear-validated
single-excursion direction logic this generalizes per-region, and the τ_hi/τ_lo/τ_contrast seeds);
`docs/design-s26-multiexcursion-keyplan-engine.md` §2.2 (`region_excursion_offset` reads the chosen
region's OWN valence/hue; hue distance measured against the SUBJECT hue) + §3 (the energy-DESCENDING
ranking of non-subject regions). Realizer/planner facts checked against `src/composition.rs:1183–1304`
(the K1 `relative_offset`/`excursion_offset`/`resolve_key_scheme`) and the struct at `:39–98`.

**Scope of K2a (what this doc validates).** K2a generalizes the K1 single-excursion direction logic
so that section B reads the rank-0 (most-energetic non-subject) region's OWN affect and section C reads
the rank-1 region's OWN affect, each travelling to a menu key chosen from THAT region's per-region
brightness + hue. Modulation is still DIRECT (no pivot — pivots arrive in K3). The menu stays
`{+7,+5,+3,−3}`; spice stays OFF (documented reserves only). The four questions below are the trained-ear
gates on that generalization.

---

## 0. Verdict summary (read first)

1. **Brightness→direction mapping: ENDORSED, with one boundary correction.** Brighter region → DOMINANT
   (+7, lift); darker region → SUBDOMINANT (+5, relax); strong hue contrast → RELATIVE (±3) — this is
   musically sound and reinforces (never contradicts) the affect/character plan. The ONLY change I
   require is the **cut-point**: the K1 build shipped a single `affect_valence >= 0.5` split; the
   pinned input-doc boundary is **`<= 0.40` → +5 (subdominant), everything above → +7 (dominant)**.
   **Correct the build to the 0.40 boundary** (with a mid band 0.40–0.60 that resolves to +7). See §1.
2. **Energy-descending = eye-travel order: ENDORSED.** Rank-0 (most energetic non-subject region) → B,
   rank-1 → C is the right "the eye stops here first" order. See §2.
3. **Two distinct destinations: ENDORSED, with one pairing to AVOID.** B and C reading different regions
   yields genuinely distinct, coherent key pairs. The cap of ≤2 distinct non-home keys is right for a
   short piece. The one pairing to flag is **B→dominant (+7) paired with C→subdominant (+5)** as the two
   excursions of a short ABAC — it splays the tonal center symmetrically in both directions with no
   intervening home anchor and reads as aimless on a DIRECT (no-pivot) modulation. Prefer a relative as
   one of the two stops. See §3.
4. **Smoothness without a pivot: the relative-involving journeys are safe to re-listen now; the +7↔+5
   opposite-pole pair is the one to expect roughness on.** See §4.

---

## 1. PER-REGION BRIGHTNESS → DIRECTION (Q1)

### 1.1 The mapping is musically sound — ENDORSED

Reading each non-subject region's OWN brightness (a per-region valence proxy) and mapping:

| Per-region signal (measured on THAT region) | Menu pick | Theory rationale — why it reinforces the affect plan |
|---|---|---|
| **strong hue contrast** vs the SUBJECT hue (`hue_dist ≥ τ_contrast`) | **relative** `relative_offset(home_mode)` (±3) | A region that is a genuinely different COLOR family reads as "the same world seen in a different light" — the relative shares all 7 pitch classes, so it recolors without leaving the collection. This is the maximal-color / minimal-disruption move, exactly right for a region the eye reads as *contrasting* rather than *brighter/darker*. |
| else **bright** region (`brightness > 0.40`) | **dominant +7** (lift) | Brightness is the same valence axis the mode/character plan reads. A bright region wants the upward, goal-directed pull of the dominant — the single most-heard tonal departure, heard instantly as "lift / forward tension." It can NEVER brighten an image the mode plan called dark, because mode and key direction read the same axis (S25 Decision 4 invariant). |
| else **dark** region (`brightness ≤ 0.40`) | **subdominant +5** (relax) | A dark/low region wants the flat-side, settling plagal pull — the "relaxation / amen" counterweight to the dominant's tension. It reinforces the sinking the mode plan already chose for a dark image. |

This is the correct generalization of the K1 logic. The crucial property — *the key-direction axis and
the mode/character axis are the SAME axis* — is preserved when the axis is read PER-REGION instead of
whole-image: a region the affect plan would color dark still gets the flat-side key, and a bright region
still gets the lift. Per-region reading makes the two excursions genuinely diverge (the whole point of
K2a) WITHOUT ever letting the key plan fight the mood. **Endorsed as musically sound.**

> **Trained-ear note on the proxy.** Brightness (luminance) is a *valence* proxy, not arousal. That is
> the right axis here: valence is the bright/dark, lift/settle direction; arousal (energy) is correctly
> spent on the RANKING (§2), not the direction. Keeping brightness on direction and energy on rank means
> the two image quantities map to the two musical decisions they each belong to — no axis is double-used.

### 1.2 The cut-point — CORRECT the build to 0.40 (not 0.50)

The pinned numeric convention (kept verbatim from `input-s25` §3 and restated in `input-s26` §3.1) is a
THREE-band valence reading that collapses to a single +5/+7 boundary:

| Band | Condition (on the region's own brightness 0..1) | Menu pick |
|---|---|---|
| **high** | `brightness ≥ τ_hi` (**0.60**) | **+7 dominant** |
| **mid** | `τ_lo < brightness < τ_hi`, i.e. open `(0.40, 0.60)` | **+7 dominant** ("go to V and come back" — never wrong) |
| **low** | `brightness ≤ τ_lo` (**0.40**, inclusive) | **+5 subdominant** |

Because HIGH and MID both resolve to +7, the operative +5/+7 split sits at **τ_lo = 0.40**: at or below
0.40 → +5; anything above 0.40 → +7. The strong-contrast branch (`hue_dist ≥ τ_contrast = 60.0°`) is
evaluated FIRST and overrides both.

**The K1 build deviated** (review-S25 note 1): it shipped a single `affect_valence >= 0.5` cut. That puts
regions in **[0.40, 0.50)** on **+5 (subdominant)** where this doc gives **+7 (dominant)** — i.e. mid-dark
regions land too settled/plagal. **I confirm the input-doc boundary of 0.40 and correct the deviation:
the +5/+7 split must be at 0.40, not 0.50.** Concretely, the endorsed predicate for `region_excursion_offset`:

```
if hue_dist >= 60.0           -> relative_offset(home_mode)   // ±3, strong contrast first
else if brightness > 0.40     -> +7   // HIGH or MID -> dominant lift
else                          -> +5   // brightness <= 0.40 -> subdominant settle
```

Boundary handling (no gaps, no overlaps), identical to S25 §3b: `≥ τ_hi` inclusive for high, `≤ τ_lo`
inclusive for low (so exactly 0.40 is LOW → +5), strict-between for mid; `≥ τ_contrast` inclusive for
contrast. **Endorsed cut-points: τ_lo = 0.40, τ_hi = 0.60, τ_contrast = 60.0°** — the S25 seeds, unchanged;
only the *implementation's* 0.50 collapse is corrected back to 0.40.

> **Why this matters to the ear.** The [0.40, 0.50) band is precisely the "mildly dim but not dark"
> regions — overcast skies, muted mid-tones, shadowed-but-not-black foregrounds. The dominant's gentle
> lift ("go to V and come back") is the safe, classic reading for these; pushing them to the subdominant
> over-settles a region that isn't actually dark, deadening the contrast K2a exists to create. 0.40 is
> the correct floor. (If the operator's re-listen prefers the more-settled feel, that is a re-listen tunable
> per §1.3 — but the SHIPPED default reconciles to the pinned 0.40, not 0.50.)

### 1.3 Re-listen tunables (NOT K2a blockers)

The thresholds remain re-listen candidates (S25 §6), unchanged: `τ_contrast` could drop toward 45.0° to
route more contrasting regions to the smooth relative; the neutral band width (0.55/0.45) could narrow to
give +5/relative more airtime. These are pure-data, zero-golden-risk, and ship as-is for K2a. The menu
offsets themselves and the `relative_offset` mode-family rule are NOT tunables — they are common-practice
tonal facts.

---

## 2. ENERGY-DESCENDING = EYE-TRAVEL ORDER (Q2) — ENDORSED

Assigning the **most-energetic** non-subject region to the **first** excursion (B) and the next to the
**second** (C) is the right order. The reasoning is perceptual-into-musical:

1. **Visual salience → first attention.** Edge energy / activity is a saliency proxy: after the subject,
   the eye's next fixation is the busiest, highest-contrast non-subject region. That region is what the
   viewer "visits" first, so it is the right source for the FIRST tonal departure (B). The next-busiest
   region is the second fixation → the second excursion (C). The mapping "energy rank == order of visual
   attention == order of musical excursion" is coherent and is the natural reading of "the eye sweeps."
2. **Energy → contrast magnitude → it belongs early.** The more energetic region is, on average, the more
   visually assertive one, and a B-section is where a short form spends its biggest contrast (the classic
   "depart boldly, then a second, subtler departure before the return"). Putting the strongest region's
   key first matches how a short ternary/rondo form front-loads its principal contrast. A weaker second
   excursion (C) as a subtler secondary departure is the right shape, not a defect.
3. **It is deterministic and stable.** Energy-descending with a stable tiebreak (band index) on near-equal
   energies (design Risk 5) keeps the ordering reproducible — the same image always sweeps the same way,
   which the no-RNG discipline requires.

**Endorsed.** I do not argue for a different order. The one perceptual caveat — that a *large, calm*
region can dominate attention by area even at low edge-energy — is real but is correctly out of K2a's
scope: edge-energy is the available saliency proxy, area-weighting is a later segmentation slice (design
Risk 4). Energy-descending is the right order for the proxy we have.

---

## 3. TWO DISTINCT DESTINATIONS (Q3)

### 3.1 Distinct, coherent key pairs — ENDORSED

B and C reading different regions gives genuinely distinct destinations, and the menu pairs are
musically coherent two-stop journeys. The good pairings (B-key → C-key), with the home C-major family
for concreteness:

| B → C pairing | Reads as | Coherent? |
|---|---|---|
| **dominant (+7, G) → relative (−3, Am)** | lift, then step into the shadow — the bright departure followed by the same-collection recolor | **Yes — a strong two-stop journey.** Two different *kinds* of move (functional lift vs collection recolor); the ear hears two distinct places. |
| **relative (−3, Am) → dominant (+7, G)** | shadow first, then the lift | **Yes.** The mirror of the above; equally coherent. |
| **subdominant (+5, F) → relative (−3, Am)** | settle, then recolor | **Yes.** A gentler journey for a darker image; the two stops stay distinct because one is functional, one is collection. |
| **relative (−3, Am) → subdominant (+5, F)** | recolor, then settle | **Yes.** Coherent; the relative and the subdominant are different enough not to blur. |

The shared property of every *good* pairing: **at least one of the two stops is the relative.** The
relative (7/7 shared pitch classes) is the smoothest move on the menu, so a journey that includes it always
has a low-friction anchor — the ear never loses the home collection entirely.

### 3.2 The pairing to AVOID — dominant (+7) paired with subdominant (+5)

The one menu pairing I flag is **B→dominant (+7, G) and C→subdominant (+5, F) as the two excursions of a
short, DIRECT-modulation piece.** Reasons:

- **Symmetric opposite poles with no anchor between them.** +7 and +5 splay the tonal center a fifth up
  and a fourth up (i.e. to the two notes flanking home on the circle of fifths). Visiting both, with no
  intervening home statement and NO prepared pivot (K2a is direct), the ear is pulled equally hard in two
  opposite plagal/authentic directions and never gets to re-cohere — it reads as *aimless* rather than as
  a journey with a destination. This is the "restless/cheap output" risk the spice tier was deferred to
  avoid, surfacing here from the legitimate menu.
- **No smooth anchor.** Neither stop is the relative, so neither shares the full collection — every seam
  changes an accidental, and on a direct modulation that compounds into two unprepared key changes pulling
  opposite ways.
- **The forms that hit this are exactly the short ones.** A `ternary_aba`-class piece returns home between
  B and C only when the form has an interior return; a `rounded_binary`-class short form may not. The
  +7/+5 splay is most exposed precisely where it's least anchored.

**Mitigation (already in the upstream input doc, no new machinery):** the §3.3 direction guard plus the
§3.2 distinctness advance largely prevent this — a bright region won't pick +5 and a dark region won't pick
+7, so a +7/+5 *opposite-direction* pair only arises when the two regions genuinely read opposite valence
(one clearly bright, one clearly dark). When they do, this is a coherence flag for the re-listen, not a
hard block: it is the one pairing where the operator should listen for aimlessness, and the cleanest fix
(arriving in K3) is the prepared pivot. **For K2a: ship it, but flag +7/+5 as the pairing to scrutinize on
the bench.** If it grates, the distinctness advance's relative-first rank order (§3.2 upstream) means
nudging one stop to the relative is the documented, in-menu remedy.

### 3.3 The ≤2-distinct-non-home-keys cap — ENDORSED for a short piece

Capping a short piece at two distinct non-home keys (B's key and C's key) is correct. Theory rationale:

- **A short form cannot earn more than two departures.** Each genuine key change needs enough time to be
  established and then left; a 2–4 minute image-piece has room for one principal contrast (B) and one
  secondary (C) before the return. A third distinct key in that span reads as channel-surfing, not travel.
- **The menu only has three non-home keys anyway** (`+7, +5, +3/−3`), so ≤2 distinct leaves genuine
  contrast available for the rare 3rd excursion (abbac) while still guaranteeing the two principal stops
  differ. Three distinct excursions is the practical ceiling (upstream §3.4) and is reserved for the longer
  multi-contrast forms, not the short ones K2a's re-listen will use.

**Endorsed.** Two distinct non-home keys is the right cap for the short pieces this slice is heard on.

---

## 4. SMOOTHNESS / COHERENCE WITHOUT A PIVOT (Q4)

K2a modulates DIRECTLY (pivots arrive in K3). The question for the operator's re-listen is which two-stop
journeys read acceptably *unprepared*, so an honest "the direction logic works" verdict isn't sabotaged by
a jump that only a pivot could fix.

**The relative is safe direct — always.** A move to the relative (±3) changes ZERO pitch classes (7/7
shared). There is nothing to prepare: the same notes simply re-center. Every journey whose stops are
{home, relative} reads cleanly with no pivot. A piece whose B or C is the relative will sound *intentional*
even in K2a. This is why the distinctness advance ranks the relative first (upstream §3.2) — it is the
safest forced fallback AND the safest unprepared move.

**The near keys are acceptable direct, individually.** A single direct move home→dominant (+7) or
home→subdominant (+5) changes exactly one accidental (6/7 shared) and is idiomatic unprepared — K1 shipped
on exactly this and passed the ear. So B→(+7) alone, or B→(+5) alone, with home on either side, reads fine.

**The roughness to expect:** a journey that strings BOTH near keys together without a home anchor between
them — the §3.2 +7↔+5 pair — is where an unprepared seam is most audible, because each seam changes an
accidental and the two changes pull opposite ways. That is the single journey whose K2a re-listen may
sound jumpy *for lack of a pivot*, not for a flaw in the direction logic. So when the operator hears it:

> **Re-listen guidance.** Judge the *direction logic* on the relative-involving and single-near-key
> journeys (those are fully prepared by being smooth, and any roughness there IS a direction-logic
> problem). Treat a jumpy +7↔+5 opposite-pole journey as EXPECTED unprepared-modulation roughness that K3's
> pivot resolves — do NOT read it as the per-region direction logic being wrong.

Summary smoothness ranking (best-to-roughest, all direct, no pivot):

| Journey shape | Pitch-class friction | Direct-modulation verdict |
|---|---|---|
| home ↔ relative (±3) | none (7/7) | **Clean** — sounds intentional unprepared |
| home ↔ one near key (+7 OR +5), home on both sides | 1 accidental, one move | **Idiomatic** — K1-proven unprepared |
| B→relative, C→near (or vice-versa) | one smooth stop anchors it | **Acceptable** — the relative stop carries the coherence |
| B→+7, C→+5 (or reverse), no home between | 2 accidentals, opposite directions, unanchored | **Expect roughness** — coherent only once K3's pivot prepares the seams |

---

## 5. DEPENDENCIES / OPEN ITEMS

- **The per-region fields must land first.** This direction logic reads each region's OWN brightness + hue.
  Those fields (`foreground_brightness`, `background_brightness`, `foreground_hue`, `background_hue`) are
  added by the K2a `pure_analysis.rs` re-surfacing (design §3.1–§3.2) and are NOT yet on
  `ImageUnderstanding` (struct ends at `affect_valence`, `composition.rs:97`). Until they land, the
  Implementer's `region_excursion_offset` honestly falls back to whole-image affect (design §2.2 shim) and
  reproduces K1 exactly — correct interim behavior; the direction logic above only becomes audibly
  per-region once the fields are populated.
- **Hue distance is measured against the SUBJECT hue** (design §2.2), circular (`min(d, 360−d)`), per the
  K1 convention (`composition.rs:1216–1218`). Each region's `*_hue` is compared to `subject_hue`, not to
  whole-image `secondary_hue` — that is the per-region generalization and is correct: the contrast that
  matters is the region-vs-subject color difference.
- **The 0.40 reconciliation is the Implementer's one behavioral correction in K2a** (§1.2). It is pure-data
  (a constant), zero-golden-risk (the realizer is untouched in K2a), and reconciles the build to the pinned
  input doc. Flag it as the deliberate correction of review-S25 note 1.
- **Spice stays OFF.** Chromatic mediants, truck-driver +1, tritone +6, parallel mode-flip — documented
  reserves only (upstream §1.5). The menu is exactly `{+7,+5,+3,−3}`; no new default targets.

---

*Doc-only. No source, test, or asset modified. The endorsed direction predicate (§1.2), the endorsed
cut-points (τ_lo 0.40 / τ_hi 0.60 / τ_contrast 60.0°), the energy-descending ranking endorsement (§2), the
distinct-pair coherence + the +7/+5 pairing flag (§3), and the no-pivot smoothness guidance (§4) are
binding trained-ear input for the Implementer to transcribe into `region_excursion_offset` +
`resolve_key_scheme`; the Implementer is the sole committer of `composition.rs`/`pure_analysis.rs`/
`assets/mappings.json`. Verified against `composition.rs:39–98,1183–1304` (the K1 direction logic this
generalizes) and the K1/S26 input docs + design §2.2/§3 at HEAD `9cd9681`.*

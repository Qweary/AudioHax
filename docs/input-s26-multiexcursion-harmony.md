# Input S26 — Multi-excursion key plan: pivots, earned homecoming, open endings (Music Theory lane)

**Author role:** Music Theory Specialist — S26 INPUT DOC. **DOC ONLY:** this file modifies no source,
test, or asset and owns no `.rs` file. It pins the harmonic rules the Implementer transcribes into
`resolve_key_scheme` + (the now-in-scope) `chord_engine.rs` pivot-insert + `assets/mappings.json`, and
gives a trained-ear judgment on each so the build ships *pleasing* values, not merely *correct* ones.
**Date:** 2026-06-16. **HEAD:** `9cd9681`.
**Builds on (do not restate):** `docs/design-s24-image-as-form-key-plan.md` (LOCKED — Decisions 1/3/5/6,
Invariant A, §2.5(a), §4 Slice K3); `docs/input-s25-k1-keyplan-harmony.md` (the K1 menu `{+7, +5, +3, −3}`,
the `relative_offset` mode-family rule, the closely-related-set definition — kept verbatim here);
`docs/review-S25.md` (the K1 PASS + the valence-cut deviation + the per-region-affect limitation).

**What expands in S26 (the operator's locked direction).** K1 (single ABA, DIRECT modulation, exact
home return) is frozen. S26 generalizes to: (1) a **real pivot / common-tone modulation** at section
seams — the deferred "K3" — so a key change is *prepared*, not spliced; (2) a **cadence that lands home**
at the structural return so the homecoming is *earned*; (3) **N distinct excursions** (ABAC travels to
two genuinely different related keys; abbac+ extends the same logic) driven by per-region affect; and
(4) a **conditional** `resolves_home` guard — an open / off-home ending becomes a *deliberate expressive
option* under named criteria, not a universal defect.

**Lane boundary (single-writer carry-over, S24 §2.5).** The Music Theory lane owns the **harmonic
tables** below: the menu offsets (unchanged from S25), the pivot-chord identities (Roman numeral in BOTH
keys), the voice-leading constraint at the hinge, the cadence-role assignment, the N-excursion ordering,
and the open-ending criteria. The Aesthetics lane owns the structural shape (catalogue schemes, the
`key_scheme` SelectTable rules, pacing). The **Implementer is the sole committer** of `composition.rs`,
`chord_engine.rs`, `mapping_loader.rs`, and `assets/mappings.json`; this doc is file-disjoint input only.

**Ground truth verified against the working tree (not trusted from prose):**

- The transpose seam is a uniform pre-voicing pitch-class rotation: `tonic_pc = ((home_root_midi +
  key_offset_semitones).rem_euclid(12))` (`chord_engine.rs:2105`). Home root pinned 60 = C
  (`composition.rs:937`). Confirmed.
- The realizer does **not** read `Section.boundary_cadence` to choose the cadence. `build_phrase_plan`
  (`chord_engine.rs:~620–758`) re-derives the cadence from PHRASE structure: at the FINAL phrase
  boundary it stamps a **PAC only if the immediately-preceding chord is a dominant**
  (`is_dominant_name`, `chord_engine.rs:712–717`); otherwise it falls back to a **half cadence** on V
  (`:724–728`). Non-final boundaries are always half cadences (`:730–733`). **This is the single most
  load-bearing fact for §2 and §1:** "land home on a PAC" is realized by guaranteeing a real **V→I in
  the home key at the final phrase boundary**, not by a data label.
- The six modes are exact diatonic scales (`chord_engine.rs:11–26`); unknown mode → Ionian fallback
  (`:191`). Roman-numeral degree index map: I=0, ii=1, iii=2, IV=3, V=4, vi=5, vii=6
  (`chord_engine.rs:92–106`). The realizer can build a secondary/applied dominant of any next chord
  (`secondary_dominant_of`, `:351`) and a borrowed minor iv (`:391`) — so a triad on an arbitrary
  scale degree is already within its vocabulary.
- `abac` in `assets/mappings.json:104–108` is A:Statement/**Half**, B:Contrast/**Imperfect**,
  A:Return/**Half**, C:Coda/**Perfect**. The `abac_rondo` key scheme (`:172–176`) is A:home /
  B:`region_related:b` / A:home / **C:`region_related:c`** — i.e. **C currently travels and does NOT
  return home**, which directly contradicts design Invariant A ("C's Coda resolves to the HOME key").
  **§2.4 + the §5 table fix this; this is the one substantive correction S26 makes to the locked data.**

---

## 0. Executive summary (read first)

S26 keeps the **K1 menu unchanged** — `{+7 dominant, +5 subdominant, +3/−3 relative}`, `relative_offset`
mode-family-aware — and adds three theory layers on top:

1. **Pivot / common-tone modulation (§1).** For each menu target, there is a single shared diatonic
   chord ("the pivot") that is functional in BOTH home and target. Insert it at the seam, immediately
   before the new section's first chord, and the ear hears a hinge instead of a splice. The pivots are
   conservative (real common chords, full triad overlap or strong common-tone overlap), spice OFF.
2. **Earned homecoming (§2).** Because the realizer only stamps a PAC when a **dominant precedes the
   tonic at the final phrase boundary**, the return is made earned by inserting the **home dominant (V
   of home)** as the pivot *back* from the excursion, so the recap opens on / approaches a real V→I in
   the home key. A pivot back through V-of-home is also the smoothest re-entry.
3. **Generalized N-excursion distinct-key selection (§3).** B and C each pick a menu key from
   per-region affect; a **rank-ordered distinctness rule** guarantees they don't collapse to the same
   key, and a **direction-from-affect** rule (rising→dominant, flat-side→subdominant, strong-contrast→
   relative) keeps every excursion reinforcing the mode/character plan — never brightening an image the
   affect plan called sad. The same rule extends to a 3rd+ excursion.
4. **Conditional homecoming (§4).** An open / off-home ending is *coherent* under explicit criteria
   (modal-final close, held truck-driver lift at a final boundary, a question-form half-close) and
   *broken* otherwise. This frees the `resolves_home` guard to become **conditional on the form/affect
   profile** rather than universal.

Conservative defaults throughout: clean pivots, real common tones, mode held constant in v1 (Decision 6).
Chromatic mediants / tritone / parallel-mode-flip stay **OFF**, documented as reserves only (§1.5).

---

## 1. PIVOT / COMMON-TONE MODULATION at a section seam (the deferred K3, theory side)

### 1.1 The principle (why a pivot, stated in theory)

A direct modulation to a closely-related key is *idiomatic* (K1 shipped on exactly that), but a trained
ear hears the splice the moment the seam falls on a structurally weak beat or between two chords that
share no function. A **pivot chord** (a.k.a. common chord) is one triad that is diatonic — and
functionally sensible — in BOTH the old key and the new. Sounding it once at the boundary lets the ear
**reinterpret** the chord: it enters the chord as "scale-degree X in home" and leaves it as "scale-degree
Y in target," and the modulation is heard as a pivot on that reinterpretation rather than as a cut. This
is the single most conservative, common-practice-correct way to prepare a key change. The constraint that
makes it safe is the K1 menu itself: every target is closely related (6/7 or 7/7 shared pitch classes),
so a genuine common chord ALWAYS exists and is easy to find.

### 1.2 The deterministic recipe the realizer transcribes

> **To modulate home→target at a section seam:** insert exactly ONE pivot chord as the LAST chord of the
> outgoing section (on the seam beat), then let the new section open on its own first chord.
>
> `… [outgoing section chords] , PIVOT(home→target) , [target tonic / first chord of new section] …`
>
> The pivot is `numeral_a` (its function in home) `=` `numeral_b` (its function in target). The realizer
> builds the pivot as a diatonic triad rooted on the named scale degree **of the home scale** (so it is
> spelled from notes the ear already owns), then the next section's transpose seam re-centers everything
> to the target tonic. Because the pivot is, by construction, also diatonic in the target, no chromatic
> spelling is needed.

The pivot is reachable **only** when a non-`home_only` scheme is active AND a `pivot: true` flag is set
(the K3 opt-in named in design Decision 5 / §4). The identity path inserts nothing — byte-freeze holds
exactly as in K1.

### 1.3 The pivot table — one conservative pivot per menu target (home in MAJOR family)

Pivots chosen for **maximum functional safety**: a pre-dominant or tonic-area chord shared by both keys,
landing on a strong function in the target. Roman numerals are home-relative on the left, target-relative
on the right; the realizer roots the triad on the **home** scale degree shown.

| Target (offset) | Pivot chord | = numeral in HOME | = numeral in TARGET | Why this pivot (theory) |
|---|---|---|---|---|
| **Dominant +7** (home C → G) | **C-major triad** (home tonic) | **I** (tonic) | **IV** (subdominant) | Home's I is the target's IV — a pre-dominant in the new key; the ear leaves on "home" and arrives on "subdominant of the dominant key," the classic I=IV pivot up a fifth. 6/7 shared; one new leading tone (F♯) appears only in the target's V. |
| **Subdominant +5** (home C → F) | **C-major triad** (home tonic) | **I** (tonic) | **V** (dominant) | Home's I is the target's V — the strongest possible arrival function in the new key (it *is* the new dominant), so the target tonic that opens the next section lands as a textbook V→I. The plagal-side mirror of the +7 pivot. |
| **Relative −3** (C major → A minor) | **C-major / A-minor shared diatonic** — use **A-minor triad** | **vi** (submediant) | **i** (tonic) | The two keys share ALL seven pitch classes, so any diatonic triad pivots; the cleanest is home's **vi**, which is literally the target's tonic — the ear simply re-hears vi as the new "one." Zero accidentals change. |
| **Relative +3** (minor home → relative major) | **(home tonic minor) i** = target **vi** | **i** (tonic) | **vi** (submediant) | Mirror of the above from a minor home: home's i is the relative major's vi; arriving on the relative-major tonic re-hears that vi→I. 7/7 shared, zero accidentals. |

### 1.4 The voice-leading constraint at the hinge (the rule the realizer obeys)

The pivot must be voiced so the move into the **new section's first chord** is smooth. Two conservative
rules, in priority order:

1. **Common-tone retention (primary).** At least ONE voice that sounds in the pivot must be HELD (same
   pitch class, ideally same register) into the new section's first chord. Every pivot in §1.3 shares
   ≥1 common tone with the target tonic by construction (they are a fifth/third apart, never a tritone),
   so this is always satisfiable. The held tone is the hinge the ear latches onto — it is what turns a
   "jump" into a "pivot."
2. **Stepwise upper-voice motion (secondary).** Any upper voice that does NOT hold should move by step
   (≤ 2 semitones) into the new chord, never by leap. This is already inside the realizer's existing
   conservative voice-leading budget (`voice_lead_sequence`, max-leap constraint at
   `chord_engine.rs:~516`), so the pivot insert rides the existing voice-leading pass; it adds a chord,
   not a new voice-leading rule.

> **Theory rationale.** The common tone is the acoustic anchor of a smooth modulation — it is the note
> the ear carries across the seam unchanged, so the new key feels *reached* rather than *cut to*.
> Stepwise resolution of the remaining voices is the standard conservative voice-leading floor; together
> they guarantee no voice leaps across the boundary, which is precisely the "splice" artifact the pivot
> exists to remove.

### 1.5 Reserves (OFF by default — documented, not built in S26)

Chromatic-mediant pivots (a single common tone, no shared key), the truck-driver +1 bump, the tritone
+6, and parallel-mode-flip (+0 with a mode change) all need either a chromatic pivot or a common-tone-only
hinge. They are real techniques but they are spice: they trade safety for color and they belong behind an
explicit JSON opt-in beyond the v1 menu. **Keep them off in S26.** (One exception is *consumed* in §4: a
deliberately *unprepared* truck-driver lift at a FINAL boundary is itself an open-ending device — see
§4.3 — but it is gated identically and is not a default.)

---

## 2. CADENCE-TO-LAND-HOME — making the structural return EARNED

### 2.1 The realizer reality that dictates the rule

The realizer does not honor `Section.boundary_cadence` when choosing the audible cadence; it re-derives
it from phrase structure and stamps a **PAC only when a dominant immediately precedes the closing tonic
at the FINAL phrase boundary** (`chord_engine.rs:709–722`). Therefore "land home on a PAC" is not a data
assertion — it is a **harmonic precondition**: the home key's **V must sound immediately before the home
I at the piece's final phrase boundary**. The earned homecoming is realized by guaranteeing that V→I.

### 2.2 The recipe: pivot BACK to home through home's dominant

When the section preceding the structural Return is an excursion (B or, in ABAC, the second-excursion C
when it is NOT the final section), modulate excursion→home with the pivot that lands on **home's V**, then
let the Return open and drive to the home tonic:

| Re-entry (excursion → home) | Pivot chord | = numeral in EXCURSION key | = numeral in HOME | Why |
|---|---|---|---|---|
| from **Dominant +7** (G → C) | **G-major triad** (the excursion tonic) | **I** (tonic of G) | **V** (dominant of C) | The dominant key's tonic IS home's V — the single most natural re-entry; you are already sitting on home's dominant, so the recap's V→I is set up for free. |
| from **Subdominant +5** (F → C) | **C-major triad** (= IV of F) | **V**? no — use **C** as IV→ pivot to home I; or insert **G (V of C)** before the return | **V** (home dominant), reached via IV–V | The subdominant key does not contain home's V tonic as its own tonic; the conservative re-entry is IV(home)→V(home)→I(home). Insert home's **V** as the last pre-return chord so the PAC precondition holds. |
| from **Relative −3/+3** | **the shared diatonic that = home's V** (G major in C; the relative shares the collection) | (diatonic in the relative) | **V** (home dominant) | The relative shares all seven pitch classes, so home's V is already diatonic in the relative key; sound it at the seam and the return PACs cleanly with zero new accidentals. |

> **The one binding requirement:** whatever the excursion, the chord **immediately before the home tonic
> at the final phrase boundary is home's V** (a real dominant, with its leading tone). That, and only
> that, is what makes `is_dominant_name` true and lets the realizer stamp the PAC. The pivots above are
> the smoothest ways to arrive at that V; the V→I itself is the earned homecoming.

### 2.3 Half-vs-perfect cadence assignment per section role (multi-excursion forms)

Holds for `ternary_aba`, `rounded_binary`, `abac`, `abbac`, `theme_and_variations`. The principle: **home
sections that are NOT the final return close on a question (Half); excursion/contrast sections close on a
Half so the ear wants home; the final return closes on the earned PAC.** This matches the existing
`form_catalogue` data where it can, and §5 flags the one place the data must change.

| Role | Position | Cadence at its boundary | Theory |
|---|---|---|---|
| **Statement (A, opening)** | non-final | **Half** (or Imperfect) | An opening that closes Half/IAC leaves the form open — the "antecedent" question. |
| **Contrast (B)** | non-final | **Half** | The excursion rests on its own dominant (or pivots toward home's V); the open half-cadence is the door the recap closes. |
| **Contrast (B′ / Development)** | non-final | **Half / Deceptive** | A deceptive close (V→vi) at an interior boundary heightens the want-for-resolution; reserved for abbac's B′. |
| **Return (A / A′)** | **if final** | **Perfect (PAC), home key** | THE earned homecoming: V(home)→I(home) per §2.2. |
| **Return (A)** | if NOT final (e.g. ABAC's A before C) | **Half** | ABAC's interior A-return is a *partial* homecoming; it pivots home and rests Half, then C departs again. (This is why ABAC's earned-return guarantee is softer than ternary_aba's — see Invariant A handling in §2.4.) |
| **Coda (C)** | **if final** | **Perfect (PAC), home key** — when the form ENDS home (default) | Invariant A: C's Coda resolves to the HOME key even though it is new material; the final PAC is in HOME, reached by pivoting C→home through home's V. |
| **Coda (C)** | if final **and** the form/affect profile selects an OPEN ending | **Half / Plagal / modal-final** (NOT a home PAC) | The conditional case — see §4. Default is the home PAC; open is the opt-in. |

### 2.4 Invariant A correction (the substantive data fix)

The locked design (Decision 1, Invariant A) requires **ABAC's C to resolve to the HOME key** so the key
lands even though the theme is new. The shipped `abac_rondo` scheme instead gives C `region_related:c`
(C travels and stays away — Invariant-A violation, confirmed against `assets/mappings.json:175`). **The
Implementer must change the `abac_rondo` C row from `region_related:c` to `home`** so the final Coda
returns home (offset 0) and PACs in home per §2.3.

This raises a real tension: design §2.5(a) AND the operator's S26 direction both say ABAC should reach
**two genuinely DIFFERENT related keys**. If C returns home, where is the second distinct excursion?
**Resolution (theory):** the second distinct key lives in the **interior** of ABAC, not in the final Coda.
The form is `A(home) B(excursion-1) A(home, partial) C(excursion-2 THEN home)`. C is a *travelling* Coda:
it DEPARTS to a second distinct related key for its body and then **pivots home for its final phrase**,
landing the home PAC. So the journey is genuinely two-destination (B → key-1, C-body → key-2), and the
piece still ends home. Mechanically this needs a **per-section internal modulation** (C starts on
excursion-2's offset, then its final phrase returns to 0) — which is richer than one offset per section.

> **Open tension for the Architect (flagged, §6):** the current `Section.key_offset_semitones` is ONE
> offset per whole section, so a Coda that travels-then-returns cannot be expressed by a single i8. Two
> conservative options: **(a)** keep one-offset-per-section and make C's offset `home` (0) — ABAC then
> travels to ONE distinct key (B) and C is a home-key Coda with new material (still satisfies Invariant A
> and "new theme, home key," but is a single-excursion form); **(b)** add an optional
> `internal_return: true` on the Coda section so C departs to excursion-2 and its FINAL phrase pivots
> home — true two-destination ABAC. **Recommendation: ship (a) for the first S26 slice** (it satisfies
> Invariant A with no new field and is unambiguously correct), and stage (b) as the very next slice once
> the per-section internal-modulation seam exists. The two-destination distinctness logic in §3 is
> written so it applies to whichever the Architect picks (it governs B-vs-C key selection regardless).

---

## 3. GENERALIZED N-EXCURSION DISTINCT-KEY SELECTION

### 3.1 Direction from affect (unchanged from S25, restated as the per-excursion rule)

Each excursion picks its menu key from its region's affect, using the exact S25 direction logic so the
key plan ALWAYS reinforces the mode/character plan and never brightens a sad image:

| Region affect signal | Menu pick | Reinforces |
|---|---|---|
| **strong hue contrast** (circular `|subject_hue − region_hue| ≥ τ_contrast`) | **relative** (`relative_offset(home_mode)`, ±3) | the "different-but-still-related" shadow; 7/7 shared |
| else **high valence** (`≥ τ_hi`) | **+7 dominant** | bright image → major/Scherzo/Hymn + dominant lift |
| else **low valence** (`≤ τ_lo`) | **+5 subdominant** | dark image → minor/Lament/Nocturne + flat-side settle |
| else **mid** | **+7 dominant, gently** | the classic "go to V and come back," never wrong |

`τ_hi = 0.60`, `τ_lo = 0.40`, `τ_contrast = 60.0°` (S25 seeds). **Note the S25 review's recorded
deviation:** the K1 build collapsed the three bands to a single `affect_valence > 0.40 → +7` cut, so the
mid band already goes to +7 — consistent with the table above for the +5/+7 split. Keep that; it is the
correct +5/+7 boundary. The per-region-affect limitation (review note 2) still holds: until per-region
brightness/hue is first-class, "direction" reads whole-image `affect_valence` and the non-subject
`secondary_hue` — see §6.

### 3.2 The distinctness rule (the new S26 machinery)

Two (or more) excursions must not collapse to the same key, or the "two journeys" become one. Rule:

> **Excursion 1 (B)** picks its key by §3.1 from the **more-energetic** non-subject region (Decision 2:
> `background_energy ≥ foreground_energy ? background : foreground`). **Excursion 2 (C)** picks by §3.1
> from the **other** region. **If C's pick == B's pick**, advance C to the next entry in the
> **smoothness-ranked menu** (below) that is ≠ B's key. A 3rd+ excursion repeats: advance past every
> already-used key to the next-ranked distinct entry.

**Smoothness-ranked menu (the advance order):** `relative (±3, 7/7 shared) → dominant (+7, 6/7) →
subdominant (+5, 6/7)`. Ranked by shared-pitch-class count first (the relative is the smoothest, so it is
the safest fallback when forced to differ), then dominant before subdominant (the dominant is the more
idiomatic departure). The advance is **deterministic** (no RNG): it walks this fixed list, skipping any
key already claimed by an earlier excursion, and takes the first survivor.

> **Theory rationale for the rank order.** When two excursions are forced apart, the *fallback* key
> should be the one that re-coheres most easily — that is the relative (it changes no pitch classes), so
> putting it first in the advance order means a forced second key is always the smoothest available
> alternative, not a jarring one. Dominant outranks subdominant because the lift is the stronger, more
> goal-directed departure, which is what a second, escalating excursion (the rondo sweep) wants.

### 3.3 The direction guard (never fight the affect plan)

The advance in §3.2 walks the menu by *smoothness*, but it must still respect direction so a forced
second key never contradicts the affect. Constraint: **the advance may move between the three menu
entries, but it must never pick the +7 dominant for a region the affect plan classed LOW valence, nor
the +5 subdominant for a HIGH-valence region.** Concretely: a low-valence region's advance order is
`relative → subdominant` (dominant excluded); a high-valence region's advance order is
`relative → dominant` (subdominant excluded); a mid region may use the full `relative → dominant →
subdominant`. This guarantees the key plan can never brighten an image the affect plan called sad, even
under a forced-distinctness advance.

> **Theory rationale.** The whole point of reading the SAME valence axis as the mode plan (S24 Decision
> 4) is that mode and key direction cannot disagree. The distinctness advance must inherit that
> guarantee, or a tiebreak could silently put a brightening dominant on a Lament — exactly the failure
> the design forbids. Excluding the wrong-direction near-key from each region's advance list preserves
> the invariant.

### 3.4 Extending to a third+ excursion (abbac and beyond)

`abbac` has TWO contrast sections (B, B′) plus a Coda (C). The same logic extends with no new rule: each
contrast/coda section that carries a `region_related:*` rule picks by §3.1 from its assigned region,
runs the §3.2 distinctness advance against every already-claimed key, and obeys the §3.3 direction guard.
With only three menu keys, **at most three distinct excursions are possible**; a fourth would be forced
to repeat. **Recommendation:** for forms needing >3 distinct excursions, do NOT invent new menu keys
(spice stays off) — instead let later excursions REUSE an earlier key but vary by *cadence/density*
(the `contrast_actually_contrasts` invariant already accepts contrast in any of {key, density, cadence}),
or stage the spice menu as a separate ear-gated slice. For B′ in abbac specifically, the cleanest reading
is **B′ = a varied/fragmented return to B's key** (it is `Fragmented` in the form data), so B and B′ share
a key by design and the distinct second journey is C — this needs no distinctness advance at all.

---

## 4. WHEN AN OPEN / OFF-HOME ENDING IS COHERENT

The K1 `resolves_home` guard asserts the FINAL section's offset is 0 for every form. S26 makes that guard
**conditional**: an off-home ending is allowed *only* when it is one of the named coherent forms below.
The distinction is **not** "did it return home" — it is **"does the ending sound intentional or
abandoned."**

### 4.1 The one-line criterion

> An open / off-home ending is COHERENT iff the final gesture is itself a closed, stable *cadential
> statement* in some key (a modal-final, a held truck-driver tonic, or a clear half-close used as a
> question) — i.e. the piece STOPS on a chord the ear reads as a deliberate resting point; it is BROKEN
> iff the final gesture is mid-phrase, unresolved-dissonant, or simply the last chord of a section that
> never cadenced (a piece that "ran out" rather than "ended").

### 4.2 The three coherent open-ending types (what makes each sound intentional)

| Type | What it sounds like | The cadence/gesture | When it WANTS this |
|---|---|---|---|
| **Modal final** | the piece ends on its tonic but in a modal color, with NO leading-tone V→I — a plagal or sus-resolved close | a **Plagal cadence** (IV→I) or a bare tonic with a held common tone; no dominant | low-arousal, modal-character images (Dorian/Mixolydian/Aeolian Nocturne/Lament) where a leading-tone PAC would sound too "classical/closed" for the mood |
| **Held truck-driver lift** | the piece modulates UP at the final section and STAYS there, ending firmly in the new (higher) key — the gospel/pop final-chorus bump | a real **PAC in the LIFTED key** (the lift is earned in its own key; it just isn't home) | high-arousal, high-valence, bright images (Scherzo/Hymn) where the lift IS the emotional climax and returning home would deflate it |
| **Question close** | the piece ends on a half-cadence-feel — resting on the dominant, deliberately unresolved | a **Half cadence** held as the final gesture, with a clear phrase boundary so the ear knows it is the END, not a gap | ambiguous/open-affect images (low fg/bg contrast, neutral valence) where the *unanswered question* is the expressive content |

The shared property: **each ends on a chord the ear accepts as a stopping point** — a tonic (modal/lifted)
or a dominant *framed* as a deliberate question. None ends mid-phrase or on an unprepared dissonance.

### 4.3 The broken cases the conditional guard must STILL reject

- A final section that ends **Interior** (mid-phrase) — the realizer's phrase model would not stamp any
  cadence; the piece literally stops mid-thought. **Always broken.**
- A truck-driver lift that does NOT cadence in the lifted key (the offset moved but the lifted-key V→I
  never sounds) — that is a splice, not a held lift. **Broken** unless the lifted key PACs.
- An excursion key that is simply the last section with no return AND no in-key cadence — "ran out of
  sections." **Broken.**

### 4.4 Which forms/affect profiles WANT home vs MAY end open

| Form / profile | Ending default | May end open? |
|---|---|---|
| `ternary_aba`, `rounded_binary` (the returning forms) | **home PAC** (earned return) | **No** — these forms ARE the departure-and-return promise; an open ending breaks their contract |
| `abac` (Coda form), `abbac` | **home PAC** (Invariant A) | **Optionally** — a Coda is *new material*; an open Coda (modal final or question close) is a legitimate stylistic choice for low-arousal/neutral profiles |
| `theme_and_variations` | **home PAC** on the final variation | **Optionally** — a final variation that ends modal/open is idiomatic (the "fade/dissolve" ending) |
| **high-valence + high-arousal** (Scherzo/Hymn) on a Coda/T&V form | — | **MAY** take the held truck-driver lift (§4.2) — the climactic-lift ending |
| **low-arousal + modal character** (Nocturne/Lament, Dorian/Aeolian/Mixolydian) | — | **MAY** take the modal-final close |
| **neutral / low-contrast** (low fg/bg contrast, mid valence) | — | **MAY** take the question close |

> **What this frees:** `resolves_home` becomes `resolves_home_OR_coherent_open` — it asserts the final
> offset is 0 **UNLESS** the form+affect profile selected a §4.2 open ending AND the §4.3 broken-cases
> check passes (the final section genuinely cadences in *some* key). The returning forms keep the strict
> guard; the Coda/T&V forms get the conditional one. **Conservative default: every form ends home unless
> an explicit `open_ending` opt-in fires** — so the strict K1 behavior is the shipped default and the
> open ending is the deliberate, ear-gated option.

> **Trained ear.** The line between "intentional open" and "broken" is exactly whether the LAST chord is
> a stable resting point. A held tonic in any key, or a dominant clearly framed as a question, reads as a
> choice; a section that stops on an interior beat reads as a crash. The three named types are the only
> ones I would let past the guard in v1 — modal-final for the quiet/modal moods, the held lift for the
> bright climaxes, the question close for the genuinely ambiguous images. Everything else returns home.

---

## 5. PER-MENU PITCH-CLASS / NUMERAL TABLE (the Implementer transcribes)

Numeric conventions identical to `input-s25` (`{+7, +5, +3, −3}`, `relative_offset` mode-family-aware,
`home_root_midi = 60` pinned). Pitch classes shown from home C for concreteness; the realizer computes
`tonic_pc = (60 + offset).rem_euclid(12)`.

### 5.1 Home in MAJOR family (Ionian / Lydian / Mixolydian; `relative_offset = −3`)

| Menu entry | Offset | Target tonic (from C) | Pivot OUT (home→target) | numeral home / target | Cadence to LEAVE | Pivot BACK (target→home) | Cadence to RETURN |
|---|---|---|---|---|---|---|---|
| **Dominant** | **+7** | G | C-major triad | **I** / **IV** | Half (rest on target's V before the seam) | G-major triad (= target I = home V) | **PAC** V(home G→C)→I |
| **Subdominant** | **+5** | F | C-major triad | **I** / **V** | Half | insert home **V** (G) as last pre-return chord | **PAC** via IV–V–I in home |
| **Relative (down)** | **−3** | A (minor) | A-minor triad | **vi** / **i** | Half (relative shares collection) | home **V** (G, diatonic in A-minor's collection) | **PAC** V(G)→I(C) |

### 5.2 Home in MINOR family (Aeolian / Dorian / Phrygian; `relative_offset = +3`)

Home tonic still pitch-class C (mode minor). Relative target is the relative MAJOR, +3 = E♭.

| Menu entry | Offset | Target tonic (from C) | Pivot OUT (home→target) | numeral home / target | Cadence to LEAVE | Pivot BACK (target→home) | Cadence to RETURN |
|---|---|---|---|---|---|---|---|
| **Dominant** | **+7** | G | C-minor triad (home i) | **i** / **iv** | Half | G triad (= target tonic = home's dominant scale degree) | **PAC** to home i (V of minor = G major with raised 3rd — the realizer's secondary/applied-dominant path supplies the leading tone) |
| **Subdominant** | **+5** | F | C-minor triad (home i) | **i** / **v** | Half | insert home **V** (G, major-quality dominant) before return | **PAC** to home i |
| **Relative (up)** | **+3** | E♭ (major) | C-minor triad (home i) | **i** / **vi** | Half (relative shares collection) | home **V** (G, diatonic over the shared collection) | **PAC** to home i |

> **Minor-home cadence note (theory).** A strict diatonic Aeolian "V" is minor (v) and does NOT contain
> the leading tone, so a bare diatonic v→i is a weak (modal) close, not a PAC. The realizer already owns
> the fix: its applied/secondary-dominant builder (`secondary_dominant_of`, `chord_engine.rs:351`) raises
> the third to make a true major-quality dominant. **For an EARNED home PAC in a minor-family piece, the
> home-dominant chord at the final boundary must be the MAJOR-quality V (raised leading tone), not the
> diatonic minor v.** If the operator's ear prefers the modal v→i close for a minor/Aeolian mood, that is
> precisely a §4.2 *modal-final* open ending — a coherent choice, not a defect. So: major-V for the
> earned PAC; minor-v (or plagal) for the deliberate modal final. The two are the closed-vs-open fork.

### 5.3 The closely-related allowlist (unchanged, restated for the test)

`smooth_keys_only` asserts every resolved non-zero offset ∈ `{+7, +5, +3, −3}`. The pivot chords above
are all diatonic triads in BOTH keys — no new pitch-class allowlist is needed for the pivots; they are
spelled from the home or target scale the realizer already builds. The pivot insert is gated behind the
`pivot: true` opt-in; with it OFF the modulation is the K1 direct move (still in-allowlist).

---

## 6. DEPENDENCIES / OPEN TENSIONS

### For the Architect (seam location + data the realizer needs)

1. **Where the pivot is inserted.** The pivot is the LAST chord of the OUTGOING section's chord sequence,
   sounded on the seam beat, BEFORE the new section's transpose seam re-centers. The Architect must
   decide whether the pivot is appended in the planner (as an extra `StepPlan` at the section boundary,
   before voice-leading) or inserted in `chord_engine` at realize time. **Theory preference:** insert it
   pre-voice-leading so the existing `voice_lead_sequence` smooths it for free (§1.4 rule 2 then comes
   for free); this is the freeze-sensitive change design Decision 5 already named.
2. **Per-section internal modulation (Invariant-A / two-destination ABAC).** §2.4 needs a Coda that can
   DEPART and then RETURN home within one section. The current one-i8-per-section model cannot express
   this. **Decision the Architect owns:** ship single-offset-per-section (C = home, Invariant A satisfied,
   ABAC = one distinct excursion) for the first S26 slice, OR add an optional `internal_return`/second
   offset on Coda sections for true two-destination ABAC. I recommend the former first (§2.4 option a).
3. **The PAC precondition is a CHORD requirement, not a label.** The realizer stamps a PAC only if a
   dominant precedes the final tonic (`chord_engine.rs:712`). Whatever inserts the re-entry pivot MUST
   guarantee **home's V is the chord immediately before the home tonic at the final phrase boundary**, and
   for minor-family homes it must be the **major-quality V** (raised leading tone via the applied-dominant
   path). If the planner appends the pivot but the phrase model lands the boundary elsewhere, the PAC
   silently degrades to a half cadence — the Architect must align the pivot insert with the final phrase
   boundary.
4. **Open-ending opt-in surface.** §4 needs a per-plan `open_ending` selector (form+affect-gated) and the
   `resolves_home` guard relaxed to `resolves_home_OR_coherent_open`. The Architect owns where that
   selector lives (a `key_scheme` field, or a separate affect-gated rule) and the byte-freeze argument
   (the default must stay strict-home so K1 goldens are untouched).

### For the Aesthetics lens (which open endings are tasteful)

5. **Open-ending taste calls.** §4.2 names three coherent open types and §4.4 maps them to form/affect
   profiles, but the *taste* call — e.g. "does a held truck-driver lift feel triumphant or cheap on THIS
   image," "is a modal final too unresolved for a Nocturne the operator wanted to feel complete" — is an
   Aesthetics + operator-ear judgment. I have set the conservative default (always end home unless an
   explicit opt-in), so the open endings ship OFF and are turned on per-profile only after the ear test.
6. **The distinctness fallback color.** §3.2 advances a forced-distinct C to the next-ranked menu key
   (relative-first). Whether that forced relative reads as a *satisfying* second journey or as an
   arbitrary swerve is an ear call; the direction guard (§3.3) keeps it from fighting the mood, but the
   Aesthetics lens should confirm on the bench that a forced-distinct second excursion sounds like a
   genuine new destination rather than a mechanical "anything but B."

### Re-listen tunables (carried + new)

- The S25 seeds (`τ_hi 0.60`, `τ_lo 0.40`, `τ_contrast 60.0°`, the +5/+7 boundary at 0.40) ride into S26
  unchanged — re-listen candidates, not blockers (S25 §6 / review note 1).
- **New S26 tunable:** whether the pivot is sounded on the FULL seam beat or as a quick approach chord —
  a duration/voicing tunable, not a key-plan change; flagged for the bench.

---

*Doc-only. No source, test, or asset modified. The menu offsets, `relative_offset` mode-family rule,
closely-related-set, pivot tables (§1.3/§5), voice-leading hinge rule (§1.4), cadence-role assignment
(§2.3), N-excursion distinctness + direction guard (§3), and open-ending criteria (§4) are binding
harmonic input for the Implementer to transcribe; the Implementer is the sole committer of
`composition.rs`/`chord_engine.rs`/`mapping_loader.rs`/`assets/mappings.json`. Realizer facts verified
against `chord_engine.rs:11–26,92–106,184–192,351,391,516,620–758,2104–2106` and `assets/mappings.json`
`form_catalogue`/`key_scheme_catalogue`/`key_scheme` at HEAD `9cd9681`.*

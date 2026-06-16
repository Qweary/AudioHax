# Design S24 — Structural Key Plan: the SONGWRITING / AESTHETICS layer of "image as form"

**Author role:** Composition & Songwriting Aesthetics Specialist — DESIGN ONLY. This document
modifies no source, test, or asset. It writes `docs/` prose and proposes `mappings.json` rows and
planner-fill behaviour as *binding shapes for a later implementer to transcribe*; it commits
nothing. Its lens is distinct from the Music Theory Specialist's: that role guarantees a modulation
is theory-*correct* (a real pivot, a clean voice-leading, a legal key); THIS role owns whether the
key plan *sounds good and feels pleasing to a listener with a trained ear* — pacing, payoff,
contrast that rewards, a return that lands. Theory-legal ≠ aesthetically satisfying; the gap is
mine.
**Date:** 2026-06-16.
**Inputs read (verified against the working tree, not trusted from prose):**
`docs/design-s21-affective-fidelity.md` (the affect→character→sound design + the Decision/Risk
conventions this doc follows); `src/composition.rs` — the `FormSpec`/`SectionTemplate`/
`ThematicRole`/`CadenceStrength` types (`:489–512, :277–310`), the `KeyTempoPlan.key_scheme:
Vec<i8>` + per-`Section.key_offset_semitones` spine (`:741–790`), the planner pinning every offset
to 0 and `key_scheme.select(u)` to the dead `"home_only"` id (`:913–1064`); `src/chord_engine.rs`
— **the realizer ALREADY reads `key_offset_semitones` and adds it to `home_root_midi` to form the
tonic pitch-class (`:2104–2105`), and motif notes are scale/key-relative "so [they] transpose
cleanly" (`:1856–1874`)**; `assets/mappings.json` `composition` block — the live `form_catalogue`
(six forms incl. `abac`, `aaba`, `ternary_aba`, `rounded_binary`, `abbac`,
`theme_and_variations`), the `form` ladder, and the **dead `key_scheme: {default:"home_only",
rules:[]}`** with all `key_offset_semitones` 0.

> Convention (carried from design-s21 and the engine reframe): the key plan lands as **data**
> (`key_scheme` SelectTable rules + a `key_plan_catalogue`), parsed backward-compatibly via
> `#[serde(default)]`; pure-Rust default; the planner fills `key_offset_semitones`/`key_scheme:
> Vec<i8>` from the selected key plan; **no realizer/engine edit** because the transposition seam
> is already wired and exercised. The "home_only" plan stays the identity/byte-freeze anchor.

---

## 0. Executive summary (read first)

Today every image renders in ONE fixed key for the whole piece — `key_offset_semitones` is locked
to 0 in the planner and the `key_scheme` ladder is empty (`"home_only"`). The *form* already
varies by image (the `form_catalogue` has six real forms, selected by a live ladder), and the
realizer **already transposes a section by its `key_offset_semitones`** — so the machinery for a
structural key plan is BUILT and proven; the planner simply never sets a non-zero offset. The
operator's "image as musical form" vision — let the image's structure drive a key plan, so the
music traces how the eye moves through the picture — is therefore a **planner-and-data slice, not a
realizer change.** That is the cheapest possible decisive win and it is the heart of this design.

**The aesthetic design, in one page:**

- **Form: keep ABAC available, but make the DEFAULT a returning form — and bind the key plan to
  the form, not invent a parallel structure.** The operator proposed ABAC; ABAC is a fine *episodic*
  form but it is the weakest at the one thing a short generated piece most needs: a **return that
  feels like home.** The single most reliable source of listener satisfaction in a short piece is
  **departure-and-return** — leave home, go somewhere, come *back* and have it feel earned. So the
  aesthetic default is **rounded binary / ternary (A B A′)**, which the catalogue already has and
  already selects; **ABAC is reserved for high-`vertical_emphasis` / episodic images** where the
  eye genuinely travels and never returns. AABA (the songwriter's workhorse) is the strongest
  *song* form and stays for wide, low-bimodality images. (§Decision 1.)

- **The key plan rides the form's section labels, not raw section index.** A key plan is a map
  `section-label → scale-degree the section sits on`. `A`/`A′`/`Return`-role sections are ALWAYS
  home (offset 0). Contrast/Coda sections take a *related* key. This guarantees the two aesthetic
  invariants no theory rule alone gives you: **(i) the piece always resolves home** (the last
  home-role section is offset 0 and carries the form's strongest cadence), and **(ii) you never
  modulate on a section the listener hears as "home."** (§Decision 2.)

- **The menu of related keys is RANKED by smoothness, and v1 uses only the top of it.** Theory
  hands a menu; aesthetics picks *which* and *how far*. v1 default destinations are the **dominant
  (+7 semitones, "brightening/lift") and the relative (±3, "the shadow / parallel-mood")** — the
  two smoothest, most idiomatic moves, each one pivot-chord away. The subdominant (−5, "relaxation")
  is the third. Remote keys (tritone, chromatic-mediant, the "truck-driver" semitone bump) are
  **documented but OFF by default** — they are the spice that, overused, makes the output sound
  cheap or restless. (§Decision 3.)

- **Direction and meaning are chosen by AFFECT, so the key plan and the existing character plan
  REINFORCE rather than fight.** A high-valence (major, Scherzo/Hymn) image lifts to the **dominant**
  (energy up); a low-valence (minor, Lament/Nocturne) image falls to the **relative/ subdominant**
  (shadow, relaxation). The S21 affect axes already exist as knobs (`Arousal`/`Valence`); the key
  plan reads them so a bright joyful image *and* its key plan both push upward, and a dark sad image
  *and* its key plan both sink. (§Decision 4.)

- **Pacing is capped so modulation is an EVENT, not wallpaper.** At most **one modulation away and
  one back** in a v1 piece (the A B A′ contour: home → related → home). Multi-trip plans
  (`abac`, `abbac`) get at most **two distinct non-home keys**, and never change key two adjacent
  sections in a row. Too-frequent modulation reads as restless/cheap; too-static is the original
  "every image sounds the same" complaint. One well-placed, well-prepared move is worth ten. (§Decision 5.)

**Recommended FIRST build slice (S25): the key plan on the EXISTING returning forms only** — wire
the `key_scheme` ladder + a `key_plan_catalogue`, have the planner fill `key_offset_semitones` from
the selected plan for `rounded_binary`/`ternary_aba`/`aaba`, with the contrast section going to the
affect-chosen related key and the return guaranteed home. Data + planner only;
`chord_engine.rs`/`engine.rs` untouched; `key_scheme:"home_only"` stays the byte-freeze identity.

---

## 1. The aesthetic decisions (each PINNED)

### Decision 1 — FORM: the aesthetic default is a RETURNING form; ABAC is reserved, not default. (PINNED)

**The operator's proposal.** ABAC — A (home, from the subject), B (related key, from the
background), C (a different related key, from the foreground) — "so the music traces how the eye
moves through the image."

**The aesthetic critique (this is the gap I own).** ABAC is a real and useful form, but it is the
form *least* able to deliver the single biggest source of satisfaction in a short generated piece.
Compare the candidates a listener's craft actually weighs:

| Form | What it does well | The aesthetic cost | Verdict for a short generated piece |
|---|---|---|---|
| **A B A′ (rounded binary / ternary)** | Statement → departure → **return that feels like home.** The return is the payoff; the contrast makes the return *mean* something. Universally satisfying, hard to get wrong. | Can feel "small" if the B is timid. | **DEFAULT.** The most satisfying-per-second shape; dramatizes "look away, look back" — which IS how the eye reads an image. |
| **AABA** (the songwriter's 32-bar) | The strongest *song* form: the hook lands twice before you leave it, the bridge (B) is a genuine departure, the final A is a homecoming with the hook now *familiar*. Memorability is built in by repetition. | Needs a real melodic hook to repeat; with a weak theme the double-A is monotonous. | **Keep** for wide, low-bimodality, "songlike" images (the catalogue already routes `aspect_ratio≥1.6 & low bimodality` here). |
| **A B A** (statement–departure–return) | Same return-payoff as rounded binary, even cleaner. | Less internal contrast than rounded binary's developed return. | **Keep** (it is `ternary_aba`); near-equivalent to the default. |
| **ABAC** | *Episodic* travel — the eye moves and never comes back; C is a fresh destination, an open ending or a coda-as-arrival. Good for panoramic/scanning images. | **No true return.** The A *recurs* but the piece ends on new material (C), so the "home" payoff is split — the recurrence of A mid-piece is a *waypoint*, not the ending. For most images this feels less resolved. | **RESERVE** for genuinely episodic images (high `vertical_emphasis` = the eye travels top-to-bottom; the live ladder already routes these to `abac`). Make C a **Coda that returns to home key** even though it is new material, so the *key* resolves even when the *theme* does not. |
| **Verse/Chorus** | Maximal memorability via chorus return; pop's default. | Needs a clear two-tier theme (verse vs chorus) the generator does not yet produce; premature. | **Defer.** No two-tier thematic engine yet. |
| **Rondo (ABACA)** | Repeated home returns = very satisfying. | Long; needs 5 sections of distinct material; over-budget for a short piece. | **Defer** (a longer-form later option). |
| **Through-composed** | Maximal image-fidelity (no repetition forced). | **No return, no memorability** — it is the *current* problem ("nothing comes back") dressed up. | **Avoid** as a default; it is the anti-pattern this whole arc exists to fix. |

**Decision: the aesthetic default form is a RETURNING form (`rounded_binary`, already the
catalogue default and already selected for the central case). ABAC is reserved for episodic
(high-`vertical_emphasis`) images, and even there its C section returns to the HOME KEY.** The
operator's subject/bg/fg→A/B/C mapping is preserved *as the key-assignment rule* (Decision 6), but
mapped onto whichever form the existing ladder selects — the key plan does not need its own form;
it binds to the form that is already chosen. **Why a return beats episodic by default:** a listener
remembers the *shape*, and the most legible, most rewarding shape in ~60–120 seconds is "I left, I
came back, and coming back felt good." ABAC's open ending is a *style*, not the safe center.

### Decision 2 — THE KEY PLAN BINDS TO SECTION ROLE, NOT INDEX. (PINNED)

**The mechanism.** A `KeyPlan` is `section-label → degree`, where `degree` is the diatonic scale
degree (or a named relation) the section's tonic sits on relative to home. The planner already
expands a `FormSpec` into labelled `Section`s carrying a `ThematicRole`
(`Statement/Contrast/Return/Development/Coda`); the key plan keys off that role:

| `ThematicRole` | Key rule | Aesthetic reason |
|---|---|---|
| `Statement` (A) | **ALWAYS home, offset 0** | The listener must hear "this is home" before any departure can mean anything. |
| `Return` (A′) | **ALWAYS home, offset 0, + the form's strongest cadence** | The resolution. This is the payoff section; modulating it would destroy the homecoming. |
| `Contrast` (B) | the affect-chosen *related* key (Decision 3/4) | The departure. This is where modulation EARNS its keep. |
| `Development` (V1/V2 in T&V) | home or a single related key, never more than one move from home | Development should *destabilize* mildly, then the final variation re-grounds. |
| `Coda` (C) | **return to home key even if new theme** (the ABAC fix) | A coda's job is arrival. New *material* is fine; a new *key* at the very end leaves the piece unresolved. |

**The two invariants this buys (neither follows from theory-correctness alone):**

> **INVARIANT A — the piece ALWAYS resolves home.** The last `Statement`/`Return`/`Coda` section is
> offset 0. A theory engine will happily write a correct modulation to the dominant and *stay
> there*; that is legal and unsatisfying. The key plan forbids it.
>
> **INVARIANT B — no modulation on a "home" section.** You never pull the rug on the section the
> listener has been taught to hear as home. (A theory engine has no concept of "the listener's
> home expectation"; it only knows the current key.)

### Decision 3 — THE RELATED-KEY MENU IS RANKED BY SMOOTHNESS; v1 USES ONLY THE TOP. (PINNED)

Theory will offer every closely- and remotely-related key. Aesthetics ranks them and ships only the
smooth top in v1. The ranking (smoothest/most-idiomatic first):

| Rank | Destination | Offset (semitones) | Affective meaning | Smoothness (shared pitch classes / pivot availability) | v1 status |
|---|---|---|---|---|---|
| 1 | **Dominant (V)** | **+7** | brightening, lift, forward tension that *wants* to come home | 6 of 7 shared; the V/IV pivot is the most idiomatic move in tonal music | **ON** (default for high-valence) |
| 2 | **Relative (vi from major / III from minor)** | **−3 / +3** | the "shadow" — same notes, opposite mood; major↔minor without changing a single pitch | 7 of 7 shared (it is the *same* collection) — the smoothest possible move | **ON** (default for low-valence / mood-shift) |
| 3 | **Subdominant (IV)** | **−5** | relaxation, settling, the "amen" pull | 6 of 7 shared | **ON** (calm/relaxing images) |
| 4 | Supertonic / mediant (closely related) | +2 / +4 | gentle colour shift | 5–6 shared | documented, **OFF** by default |
| 5 | **Parallel (major↔minor)** | 0 (mode flip) | dramatic same-tonic mood swing | same tonic, 1–3 pitch changes | documented, **OFF** (belongs to the affect/mode plan, not key plan) |
| 6 | Chromatic mediant | +3/+4/+8/+9 | cinematic, "wonder," uncanny | 2–3 shared; no diatonic pivot | documented, **OFF** (cinematic spice; later) |
| 7 | **"Truck-driver" up-a-semitone** | **+1 (at the LAST section)** | a final-chorus energy *bump* | 0–2 shared; brute-force, no pivot | documented, **OFF**, and **capped to at most ONE per piece, at a late boundary, used sparingly** — overuse is the textbook sound of cheap modulation |
| 8 | Tritone | +6 | maximal tension/alienation | 1–2 shared | documented, **OFF** (almost never tasteful by default) |

**Decision: v1 ships ranks 1–3 (dominant, relative, subdominant) only.** They are each one pivot
chord from home, so the Music Theory Specialist can always realise a smooth modulation into and out
of them; they cover lift, shadow, and relaxation — the three affective directions an image actually
needs. Everything below rank 3 is documented vocabulary the ear can switch on later, gated behind a
JSON row, never auto-selected in v1.

### Decision 4 — DIRECTION IS CHOSEN BY AFFECT SO THE KEY PLAN AND CHARACTER PLAN REINFORCE. (PINNED)

The S21 affect axes (`Arousal`/`Valence`, already knobs) pick the *direction* of the one allowed
departure, so the key plan pushes the SAME way the character/mode plan already pushes:

| Image affect | S21 character (already) | Key-plan departure (this design) | Why they reinforce |
|---|---|---|---|
| high valence (bright/major) | Scherzo / Hymn (major) | **up to the DOMINANT (+7)** | major + dominant lift = unified brightening; the music *rises* as the image *glows* |
| low valence (dark/minor) | Lament / Nocturne (minor) | **to the RELATIVE / SUBDOMINANT** | minor + flat-side move = unified sinking/shadow; nothing fights the gloom |
| mid / neutral | Ballad (default) | **to the DOMINANT (+7), gently** | the safe, classic "go to V and come back" — the most-heard tonal journey, never wrong |

This is the load-bearing integration with S21: a bright image already gets Scherzo + major + fast;
now its key plan *also* lifts to the dominant — every axis agrees. A dark image already gets Lament
+ minor + slow; now its key plan *also* drifts flat-side — coherent melancholy instead of a sad
character bolted onto a brightening key change.

### Decision 5 — PACING: modulation is an EVENT, not wallpaper. (PINNED)

Aesthetic caps, encoded so the generator cannot produce theory-valid-but-restless output:

- **At most ONE departure and ONE return** in a default (A B A′) piece: `home → related → home`.
  This is the whole point — a single, well-prepared, well-resolved trip.
- **Multi-section forms (`abac`, `abbac`) get at most TWO distinct non-home keys**, and the piece
  still ends home. `B` and `C` may differ (the operator's bg→B / fg→C two-destination idea) but
  both are drawn from ranks 1–3 and the final section resolves home.
- **Never change key two adjacent sections in a row** without a home section between, except the
  capped truck-driver bump (OFF in v1). Back-to-back modulations sound aimless.
- **Dwell time floor:** a section must be long enough to *establish* its key before leaving — the
  existing `rel_len`/`base_step_budget` sizing already gives each section enough steps; the key
  plan must not subdivide a section. One key per section, period.
- **The return must be EARNED, not just arrived at.** The contrast section should approach its
  boundary cadence (already `Half`/`Imperfect` on B in the catalogue) so the ear *wants* home; the
  return then lands on the form's `Perfect` cadence. This is already half-present in the
  `boundary_cadence` data — the key plan leans on it rather than re-inventing it.

---

## 2. Mapping image → key plan, AESTHETICALLY (Decision 6, PINNED)

**The operator's mapping:** subject→A (home), background→B (related), foreground→C (different
related). **The aesthetic refinement:**

1. **The SUBJECT should set HOME — agreed, and it already does, indirectly.** Home key/mode comes
   from the palette (`dominant_hue`→mode, the affect `valence`→major/minor). The salient subject IS
   the dominant visual mass, so "subject sets home" is the right instinct and largely already true.
   Make it explicit: when a clear subject exists (`subject_size` in a mid band, `fg_bg_contrast`
   high), the subject's hue/saturation seeds the home key (this is the same signal S21's prominence
   slice uses).
2. **Background → the FIRST departure (B) — agreed, with a direction rule.** The background is what
   the eye drifts to *after* the subject — so it is the natural "departure" destination. Its
   *energy* (`background_energy`, a real knob) and the affect valence choose the related key: calm
   background → subdominant (relaxation); busy/bright background → dominant (lift).
3. **Foreground → the SECOND destination (C) ONLY in multi-trip forms; otherwise FOLD IT INTO THE
   RETURN.** Here is the aesthetic correction to the operator's proposal: in the *default* returning
   form there is no C — and that is *better*, because a third key in a short piece is usually one
   too many (Decision 5). Reserve fg→C for the episodic `abac` case, and even there make C resolve
   to **home** (Invariant A). So the richer the image's structure (more distinct
   subject/foreground/background energy, higher `vertical_emphasis`), the more the form ladder
   already escalates rounded_binary → ABAC, and the key plan escalates one-trip → two-trip in
   lockstep.

**Is subject→A / bg→B / fg→C the most evocative assignment?** *Mostly yes, with one improvement.*
The eye reads **subject first (home), then the surrounding field.** Whether the *background* or the
*foreground* is the "second" destination depends on the image, but for the *experience* to mirror
seeing the image, the rule should be **energy-ordered, not name-ordered**: the *more energetic* of
the two non-subject regions becomes the first/more-distant departure (it grabs the eye next), the
calmer one the nearer move or the return cushion. This is more evocative than a fixed bg→B/fg→C
because it tracks *where the eye actually goes second*, which is the louder region — exactly the
"trace how the eye moves" goal, done by saliency rather than by a fixed label.

**Affect and key plan must not fight (the S21 reinforcement, restated as a rule):** the mode
(major/minor, owned by valence per S21) and the key-plan direction (Decision 4) are both read from
the same valence axis, so they are *guaranteed* to agree — a major image lifts, a minor image
sinks. The one thing to forbid: never let the key plan choose a *brightening* dominant move on an
image the affect plan has called a Lament. The shared-valence-read makes this structurally
impossible, which is the point.

---

## 3. Guard-rails for "pleasing" (the encodable aesthetic constraints)

These are the concrete constraints a later implementer encodes so the generator cannot produce
theory-valid-but-ugly output. Each is a *property*, testable by the Test Engineer.

1. **RESOLVES-HOME invariant (hard).** The final non-empty section's `key_offset_semitones == 0`.
   *Test:* over every form in the catalogue and every key plan, `plan.sections.last().key_offset == 0`.
2. **HOME-SECTIONS-ARE-HOME invariant (hard).** Every section with `ThematicRole::Statement` or
   `Return` has `key_offset == 0`. *Test:* sweep all forms × key plans.
3. **MODULATION-COUNT cap (hard).** The number of *distinct non-zero* offsets across a piece is ≤ 2,
   and the number of section-to-section key *changes* is ≤ 3 (out, [over], back). *Test:* count
   transitions in the expanded `key_scheme: Vec<i8>`.
4. **SMOOTH-KEYS-ONLY in v1 (hard).** Every non-zero offset ∈ {+7, −5, +3, −3} (dominant,
   subdominant, relative up/down). No offset outside the v1 set without an explicit OFF-by-default
   JSON opt-in. *Test:* assert the offset set ⊆ the v1 allowlist.
5. **NO-ADJACENT-MODULATION (soft→hard).** No two consecutive sections both carry non-zero offsets
   *unless* a home section separates them (truck-driver exception OFF in v1). *Test:* scan adjacent
   pairs.
6. **CONTRAST-ACTUALLY-CONTRASTS (soft).** A `Contrast` section must differ from its neighbours in
   at least one of {key offset, mode, character density} — so "B" is audibly a departure, not a
   relabel. (Mostly already true via the boundary-cadence + texture data; the key plan adds the key
   axis.) *Test:* `Contrast` section differs from the preceding `Statement` in ≥1 dimension.
7. **HOME-ONLY identity (the byte-freeze anchor).** `key_scheme:"home_only"` (the legacy default,
   and `KeyPlanMappings::default()`) yields all-zero offsets → the realizer's
   `home_root + key_offset` math at `chord_engine.rs:2105` is unchanged → **byte-identical to
   today.** *Test:* `home_only_keyplan_is_byte_identical` — a no-`key_scheme`-rules mapping
   reproduces the goldens.

---

## 4. Sliceability (one stage per session)

The realizer change is **already done** (it reads `key_offset_semitones` at `:2105`), so the entire
arc is planner + data, and it slices cleanly:

### Slice K1 — the key plan on RETURNING forms only *(RECOMMENDED S25 FIRST SLICE; the audible win)*

- **Scope:** wire the `key_scheme` SelectTable (replace the dead `{default:"home_only", rules:[]}`)
  + a new `key_plan_catalogue` (label→degree maps), and have the planner fill
  `key_offset_semitones` per section and the `KeyTempoPlan.key_scheme: Vec<i8>` from the selected
  plan, for `rounded_binary` / `ternary_aba` / `aaba` only. Contrast section → the affect-chosen
  related key (dominant for high-valence, relative/subdominant for low); Statement/Return → home.
- **Files (data + planner ONLY):** `assets/mappings.json` (`key_scheme` rules + `key_plan_catalogue`),
  `src/composition.rs` (the `KeyPlan` type + the per-section offset fill in `plan()`),
  `src/mapping_loader.rs` (the `#[serde(default)]` mirror). **`chord_engine.rs` / `engine.rs`
  untouched** — the transposition seam already consumes the offset.
- **Byte-freeze (one line):** `key_scheme:"home_only"` / `KeyPlanMappings::default()` → all offsets
  0 → `home_root + 0` at `:2105` is today's math → goldens unmoved; non-home offsets are reachable
  only through a populated key plan the equivalence net never builds.
- **What the owner hears:** *a bright image now LEAVES home for the dominant in its middle section
  and comes back — a real key change, then a homecoming — instead of one static key the whole way;
  a dark image drifts to the relative/subdominant and returns. The single biggest "this is a
  composition, not a loop" win.*
- **This is the v1-essential slice** — the audible aesthetic payoff with zero realizer risk.

### Slice K2 — the episodic (ABAC two-destination) plan + energy-ordered region mapping *(follow-on)*

- Add the two-destination key plans for `abac`/`abbac` (B and C distinct related keys, C resolving
  home), and the **energy-ordered** subject/fg/bg → departure mapping (Decision 6): the more
  energetic non-subject region becomes the more-distant departure. Same files; still no realizer
  edit.
- **What the owner hears:** *a panoramic / vertically-travelling image now genuinely journeys
  through two related keys and still lands home — the "eye moving through the image" effect, but
  resolved.*

### Slice K3 — the spice tier (OFF-by-default vocabulary) *(later refinement, ear-gated)*

- Document-and-wire (but leave disabled) ranks 4–8: chromatic mediants for "cinematic/wonder"
  images, the single capped truck-driver bump, parallel-mode swing. Each a JSON opt-in row, none
  auto-selected. Ship only after the owner's ear asks for more colour than ranks 1–3 give.

**Sequencing rationale.** K1 is the cheapest decisive win — it makes the static-key complaint
audibly gone using only the affect axes and the returning forms that are already selected, and it
de-risks the whole arc by proving the key-plan fill before any episodic complexity. K2 adds the
operator's literal subject/bg/fg multi-key journey once the one-trip version is heard and trusted.
K3 is pure spice, ear-gated.

---

## 5. Risks / trade-offs

1. **A KEY CHANGE NEEDS A PIVOT, OR IT SOUNDS LIKE A SPLICE (the load-bearing dependency on the
   Music Theory lens).** This design pins *which* key and *when* and *that it resolves home*; it
   does NOT write the modulation itself. Whether the move from home → dominant uses a proper pivot
   chord (e.g. the home IV = dominant's bVII, or a V/V secondary dominant) is the Music Theory
   Specialist's job, and a key plan with no pivot chord at the section boundary will sound like a
   tape splice no matter how tasteful the destination. **Open tension for the theory lens:** does
   the chord engine, at a section boundary with a non-zero `key_offset`, insert a pivot/applied
   chord, or does it hard-cut? v1 ranks 1–3 are all one-pivot-away by construction, which is
   *why* they are the v1 set — but the realizer must actually take the pivot.
2. **THE RETURN MUST RE-ESTABLISH HOME, not just transpose back.** Offsetting the final section to 0
   is necessary but not sufficient — the ear needs a cadential gesture (a V→I in the home key) to
   *feel* the homecoming. The catalogue's `Perfect` boundary cadence on the return section already
   provides the slot; the theory lens must ensure that cadence is in the HOME key. *Flagged for the
   theory lens.*
3. **MODE AND KEY-OFFSET INTERACT (don't double-darken or double-brighten by accident).** Valence
   owns mode (S21) AND direction (Decision 4) — so a minor image goes flat-side, which is correct,
   but if a later build *also* let the key plan pick a flat mode on the departure, the contrast
   could collapse (everything dark, no relief). v1 keeps mode constant across sections (only
   offset varies); a per-section modal plan is explicitly out of scope. *Flagged.*
4. **THE NUMBERS ARE SEEDS, TUNED BY EAR.** The offset choices (+7/−5/±3), the affect thresholds
   that pick direction, and the modulation-count caps are a principled STARTING calibration — the
   *directions and the ranking* are from common-practice tonal craft, the *exact assignments* are
   seeds. The owner's trained ear is the gate (per design-s21 Risk 4).
5. **ABAC's C-resolves-home rule fights ABAC's "open ending" character.** Some episodic images
   genuinely want an unresolved, open C (an unanswered question). v1 forces C home for safety
   (Invariant A); an opt-in "open-ending" key plan that lets C stay on a related key is a
   documented K3 option for the rare image that earns it — but it is OFF by default because an
   unresolved ending is the easiest way to make a generated piece sound broken rather than artful.
6. **mapping_loader mirror is easy to forget (carried from design-s21 Risk 7).** Every new
   `PlanMappings` field (`key_plan_catalogue`, the populated `key_scheme`) must also land on
   `CompositionMappings` + the `From` impl with `#[serde(default)]`, or the key plan is silently
   dropped at load and every piece quietly stays home. The `home_only_keyplan_is_byte_identical`
   test is the witness; add a `key_plan_round_trips` test too.

---

## 6. Recommended S25 FIRST BUILD SLICE

**Slice K1 — the structural key plan on the existing returning forms.** Data + planner only
(`assets/mappings.json` key_scheme rules + `key_plan_catalogue`; `src/composition.rs` per-section
offset fill; `src/mapping_loader.rs` mirror); **`chord_engine.rs` and `engine.rs` untouched because
the transposition seam at `chord_engine.rs:2104–2105` already reads `key_offset_semitones`.** It is
the biggest audible "this is a composition, not a fixed-key loop" win with the least risk — it makes
the static-key complaint audibly gone by letting the contrast section depart to the affect-chosen
related key and guaranteeing the return lands home, and it proves the key-plan fill before any
episodic two-key complexity. Byte-freeze in one line: `key_scheme:"home_only"` /
`KeyPlanMappings::default()` ships all-zero offsets, so the realizer's `home_root + key_offset` math
is today's and the goldens cannot move. The one thing this slice REQUIRES from the Music Theory
lens: a pivot/applied chord at the modulating boundary so the move sounds prepared, not spliced.

*Design-only. No source, test, or asset modified by this document. The proposed `mappings.json`
rows and planner fills are binding shapes for a later implementer to transcribe; bodies and the
single-writer mappings.json commit are deferred to the slice implementer, coordinated with the
Music Theory Specialist (single-writer of mappings.json) through the lead. The build role titles
(Rust Architect, Rust Implementer, Music Theory Specialist, Test Engineer, Quality Gate) are the
domain titles already used in the committed S21 docs.*

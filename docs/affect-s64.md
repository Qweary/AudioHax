# S64 STEP 2 — Affect / Cross-Modal Coherence Review of the Phrase-Period / Cadence-Scheme Spread

**Reviewer:** Perceptual / Cross-Modal Affect Specialist. **Scope:** does the feature-driven
`PhraseScheme` selection (`assets/mappings.json` → `composition.phrase_scheme`, resolved in
`composition.rs::plan`, realized by `chord_engine.rs::plan_phrases_scheme`) *reinforce* or *fight*
each corpus image's affect/character plan? READ-ONLY review; no production code inspected for
correctness (Music Theory owns that) — this is purely the affect↔structure alignment judgment.

**Anchors.** Valdez & Mehrabian 1994 (brightness→valence, saturation→arousal); Eerola / Friberg &
Bresin 2013 (affect cues combine *additively* — independent cues reinforce, they don't average).

**Verdict up front: COHERENT-WITH-NOTES.** The complexity/edge driver is affectively **sound** for
phrase structure — arguably the *right* axis, not a mismatch — and the lever does **genuine
independent affective work** (it is NOT redundant with tempo/density). Two per-image pairings carry a
mild scheme↔character tension worth naming (Img3 scherzo+group_6; magicstudio march+period_4), but
neither is a contradiction the ear will reject, and one of them (magicstudio) is actually the driver
disambiguating *correctly*. No re-seed from arousal/valence is warranted; a light arousal *modulation*
of cadence-gesture character is the clean later-slice refinement.

---

## 1. The two planes I am aligning

**Plane A — the affect/character plan** (the color→affect bridge, Valdez-Mehrabian territory):

- `composition.affect.arousal_weights` (`mappings.json:140`) = `0.45·avg_saturation + 0.25·colorfulness
  + 0.20·edge_activity + 0.10·complexity`. **70% of arousal is COLOR (saturation+colorfulness); only
  30% is structure (edge+complexity).**
- `valence_weights` (`:146`) = `0.70·avg_brightness + 0.20·avg_saturation + 0.10·fg_bg_contrast` — a
  brightness bridge.
- `character` SelectTable (`:161-170`) reads `arousal`/`valence` → picks the character, which sets the
  tempo window (`character_tempo`, `:151`) and, via `mode_valence_cuts` (`:159`,
  `composition.rs::valence_family_mode:267`), the major/minor family. Blend site:
  `composition.rs:363-364`.

**Plane B — the phrase-scheme plan** (the new lever): `phrase_scheme` SelectTable (`:374-386`) reads
`complexity` (primary) + `edge_activity` (secondary) → one of four archetypes. **100% structure, 0%
color.**

The load-bearing fact for this whole review: **Plane B's driver is largely ORTHOGONAL to Plane A's
arousal.** They share only the 0.30 edge+complexity weight; arousal is dominated by color, which the
phrase scheme never sees. That orthogonality is simultaneously the source of the lever's *value*
(§4) and the source of its two mild *tensions* (§2).

## 2. Per-image scheme ↔ affect alignment (real planner output)

Character/mode from `tests/vocab_diag_s63.rs` (real seated plan, seed 42); scheme from
`tests/phrase_scheme_s64.rs::corpus_features_select_expected_schemes`. Character encodes the arousal
band directly (`:164-168`): scherzo/march = **high** arousal (≥0.34); lament/hymn/nocturne = **low**
(≤0.30–0.35). Valence: scherzo/hymn ≥0.55, march/lament <0.55/<0.35.

| image | cx / edge | scheme | scheme "drive" reading | character | arousal | valence | tempo | alignment |
|---|---|---|---|---|---|---|---|---|
| AudioHaxImg1 | 0.005/0.301 | `period_4` | stable normative period | **Lament** | low | low | 44–66 | COHERENT |
| AudioHaxImg2 | 0.015/0.509 | `period_4` | stable normative period | **Hymn** | low | high | 60–92 | **STRONG** |
| AudioHaxImg3 | 0.229/0.475 | `group_6` | broad, *unhurried* long line | **Scherzo** | high | high | 120–168 | notes (mild) |
| example | 0.905/0.719 | `sentence_8` | one *driven* arc, accel. frag. | **March** | high | low-mid | 96–132 | **STRONG** |
| Lena | 0.164/0.471 | `phrase_5` | asymmetric off-square period | **Scherzo** | high | high | 120–168 | COHERENT |
| magicstudio | 1.000/0.106 | `period_4` | stable normative period | **March** | high | low-mid | 96–132 | notes (tension) |

**Img2 / Hymn / period_4 — strongest pairing in the corpus.** A hymn *is* the foursquare 4+4 period
(antecedent half-cadence → consequent PAC). period_4 doesn't just avoid fighting the hymn — it is the
literal phrase archetype of the character. Textbook reinforcement.

**example / March / sentence_8 — the flagship success.** The Schoenberg sentence's continuation-with-
fragmentation *is* rising urgency driving to one terminal cadence; a march is purposeful forward
motion at high arousal. Routing the highest-complexity/highest-edge image (0.905/0.719) to the most
forward-driving scheme puts the maximum musical drive under the maximum visual energy. This is the
mapping working exactly as intended, and it lands on the one image where structure AND color-arousal
both agree "high energy."

**Lena / Scherzo / phrase_5 — quietly well-aligned.** A scherzo lives on metric play and off-square
wit; `phrase_5`'s asymmetric 5+3 (interior cadence walking one step late) reads as light-footed and
slightly off-kilter — a good affective fit for a playful character.

**Img1 / Lament / period_4 — coherent, mildly under-differentiated.** A lament is low-arousal and
un-driving; the stable normative period doesn't impose false forward motion, so it doesn't *fight*.
One could argue a lament wants the broad unhurried `group_6` long line more than a square period, but
Img1 is deliberately pinned to `period_4` to protect its razor-thin F5a anti-fusion floor (design doc
§4) — an acceptable, affect-neutral hold, not a contradiction.

### The two tensions worth naming

**(a) Img3 / Scherzo / group_6 — the softest pairing.** Scherzo = quick, light, playful; `group_6` is
described as a "**long, unhurried** phrase." "Unhurried" pulls against "scherzo." Why it happens: Img3
is complexity 0.229 (low-mid structure) but its high arousal comes from *color* (saturation/
colorfulness), which the phrase driver can't see — so structure says "broad line," color says
"playful energy." **It is not a contradiction the ear rejects:** the fast scherzo *tempo* (120–168)
carries the playfulness; `group_6` just gives that fast material a longer breath before the cadence.
A fast, long-lined scherzo is musically fine. Flag it as the weakest reinforcement, not a defect.

**(b) magicstudio / March / period_4 — the one latent affect↔structure contradiction, and why it's
actually correct.** The character is **March** (high arousal → "driving"), but the scheme is the
*least*-driving `period_4`. On its face: driving affect + stable phrase = contradiction. Resolution:
the March here is a **color artifact** — magicstudio is saturation 1.0-ish (drives arousal) but
edge_activity **0.106**, the lowest in the corpus: a saturated but **motionless** field. A static
field has **no linear motion to justify musical forward-drive**. So `period_4` is affectively the
*honest* choice — you do not want an accelerating sentence over an image with zero visual motion. The
`edge_activity` secondary term is precisely what disambiguates: it holds the static image off the
driven schemes even though its color makes it read "high arousal." The tension is real (march-vs-
period), but it exposes that **edge_activity is the better proxy for "should the music drive" than the
color-saturation-loaded arousal is** — which is an argument *for* the current driver, not against it.
(The period_4 pin was independently forced by the S48 F4 register gate per design §4.1; affect happens
to agree with the freeze-safety choice.)

## 3. Is complexity/edge the right affective driver for phrase structure? — YES, defensibly

The question assumes phrase periodicity is a structural parameter that "classically" arousal (tempo/
density) and valence (mode) should drive. **My read: phrase architecture belongs on a THIRD,
distinct correspondence, and complexity/edge is the right home for it — not arousal/valence.**

- Valdez-Mehrabian bind **color → affect** (brightness→valence, saturation→arousal). The `affect`
  block already owns that bridge and spends it on tempo/character/mode. That is the color-energy plane.
- Phrase periodicity / cadence spacing is **architectural** — how the piece is *carved into units*.
  The affectively coherent visual analogue of musical architecture is the image's **architecture**:
  its spatial-frequency / detail density (`complexity`) and linear-motion content (`edge_activity`).
  "Busy, edgy visual field → more articulated, goal-driven musical structure; sparse, calm field →
  broad stable period" is a legitimate **structure→structure** cross-modal correspondence, independent
  of the color→affect bridge.
- **Re-seeding phrase structure from arousal/valence would be the wrong move.** Arousal is 70% color;
  moving phrase length onto it would (i) **collapse the phrase axis onto the tempo/character axis**
  (both then reading the same color-arousal), re-introducing exactly the cross-piece sameness S64 is
  fighting, and (ii) make the *static-but-saturated* magicstudio drive forward (high color-arousal →
  driven scheme) — the one outcome §2(b) shows is affectively wrong. The current driver gets
  magicstudio *right* precisely because it is NOT arousal-seeded.

So: complexity/edge is a **reasonable-to-superior** proxy, not a mismatch. The one caveat is
terminological: the sentence's "accelerating/urgent" quality genuinely *is* an arousal reading, so
the ideal is not "re-seed from arousal" but "let arousal **co-gate the drive**" — see §5.

## 4. Reinforcement vs. redundancy — the lever does independent affective work

Because the phrase driver (complexity+edge, 100% structure) is orthogonal to arousal (70% color), the
phrase scheme is **not** a proxy for tempo/density. Concretely, the corpus shows the two axes
*disagreeing* — Img3 (high color-arousal, low structural complexity → fast tempo BUT broad phrase) and
magicstudio (high color-arousal, zero edge → driving character BUT stable phrase). If phrase structure
merely tracked arousal, those disagreements couldn't exist; they prove the lever is reading a
**different dimension of the image** than the tempo/character ladder is. Under the Eerola / Friberg-
Bresin additive-cue model, that is exactly what you want: an **independent cue** that *adds* a
distinctness dimension (structural-busyness → phrase-architecture) on top of the color→affect cue,
rather than doubling an existing one. This is the lever earning its place — it breaks the "Wizard of
Oz" sameness along an axis nothing else in the plan was using.

## 5. Recommendation

- **Keep complexity/edge as the phrase-LENGTH / periodicity driver.** It is affectively sound, it does
  independent work, and it disambiguates the static-saturated case correctly. Do **not** re-seed phrase
  selection from arousal/valence — that would collapse it into the tempo axis and undo the win.
- **Later slice (the deferred S65 cadence-GESTURE catalogue is the natural home):** let **arousal/
  valence lightly MODULATE cadence-gesture character**, not phrase length. Two concrete folds:
  1. Gate the *driven* reading of the sentence on arousal as a secondary confirm (edge AND high
     arousal), so the accelerating gesture lands only where structure and affect agree it should —
     `example` already satisfies both; this formalizes why it's right and guards future images.
  2. On low-valence characters (lament/march), bias the cadence gesture toward a *falling/feminine*
     ending; on high-valence (hymn/scherzo), toward a *lifted* one — a valence→gesture cue that
     reinforces mode without touching periodicity.
- **Doc-hygiene nit (not affect-domain, flagging in passing):** `mappings.json:374` `_note` states
  "magicstudio(1.000,0.106)→**group_6**", but rule ordering (`complexity>=0.85 → period_4` at `:381`
  fires first), the design doc §3/§4.1, and `phrase_scheme_s64.rs:330` all resolve magicstudio to
  **period_4**. The `_note` text is a typo; the code and tests are correct.

**Bottom line:** the phrase-period lever is affectively **COHERENT-WITH-NOTES** — two mild pairings
(Img3, magicstudio) worth an ear-check but neither ear-rejecting, one of them (magicstudio) evidence
the driver is working; the complexity/edge axis is the *right* home for phrase architecture and should
**stay**, with arousal/valence reserved for a later cadence-gesture *modulation* rather than a re-seed.

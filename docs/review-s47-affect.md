# S47 Slice 1 — Affect / Cross-Modal Review: SIZING the Figure-Ground Hierarchy Knobs

REVIEW / RECOMMENDATION DOCUMENT (Perceptual / Cross-Modal Affect lens). **No
source, test, or asset file was changed to produce it** — this is the standing
TASTE/AFFECT review voice wired into the S47 build cadence (per the Specialist
Marshaling Gate). Its job: SIZE the ear-tunable magnitudes of the just-landed
figure-ground build, name the single biggest perceptual risk, and hand the
operator a concrete A/B listen-for checklist. I reason from cross-modal /
auditory-scene-analysis research + the scorecard data; **I cannot hear the WAVs —
the operator is the final ear.** The recommendations below are starting A/B
anchors, not verdicts.

**Grounded against the live tree (read this session, cited file:line):**
`src/engine.rs` BYTE-FROZEN at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-verified
UNCHANGED). The build landed exactly to `docs/spec-s47-slice1-build.md`:
`ActivityClass` + `melody_activity_class` (`chord_engine.rs:1051/1073`), the
governor (`:2232-2270`), the melody activity floor (`:2300-2302`), the seat-order
guard (`:1544-1559`), the image-conditioned prominence family
(`mappings.json:365-403`), and — landed in the same engagement as Slice 4's bed
recession — `pad_onset_cap` / `melody_min_onsets` / the Pad thinning
(`chord_engine.rs:1177-1213`). Prior lens: `docs/design-s46-affect.md` (the
cue-strength ranking rhythm-grid > onset > register > articulation > LEVEL).
Metrics: `docs/spec-s46-figure-ground-metrics.md` (F1–F5).

**The pinned per-image figure-strength axis** (from the live routing comment
`mappings.json:343`, the feature-distribution probe over the 6-image set):

| Image | `fg_bg_contrast` | counter routed? | prominence tier it selects | figure-strength class |
|---|---|---|---|---|
| AudioHaxImg1 | **0.341** | yes (`pad_bed_counter`) | `melody_lead_strong` (DEEP) | SUBJECT |
| AudioHaxImg2 | **0.284** | yes | `melody_lead_strong` (DEEP) | SUBJECT |
| AudioHaxImg3 | **0.203** | yes | `melody_forward` (MID) | MID |
| example.jpg | **0.136** | no | `melody_forward` (MID) | MID |
| magicstudio | **0.084** | no | `melody_lead_gentle` (SHALLOW) | FIELD |
| Lena | **0.052** | no | `melody_lead_gentle` (SHALLOW) | FIELD |

This table is the load-bearing context for knobs 3 and 4 — it is the actual bin
assignment the live thresholds produce, and it surfaces one perceptual
mis-bin (AudioHaxImg3, see knob 4).

---

## 0. The one finding that reframes the sizing (read before the knobs)

**The measured F5b/F1 win is PAD-onset-count driven, not counter driven** — and
this changes how I size the deep recession. The build comment is explicit
(`chord_engine.rs:1112-1125`): the live F5b violations were the **Pad**
out-onsetting the melody, because `onsets_per_step` counts *every* Pad
`NoteEvent`, so a 3-voice block stab at offset 0 reads as **3 onsets** by the
metric's lens **even though the ear hears ONE attack**. So the "deep bed thinned
to ~1 onset under a ~2-onset melody, margin +1.0 to +1.25" result the prompt
hands me is a **metric-space** statement about NoteEvent counts, not a literal
perceptual statement that the bed is reduced to a single audible event.

This is the crux of the whole affect review: **the F1 margin in onsets/step
OVER-READS the perceptual sparseness of a block bed.** A block Pad capped to 1
NoteEvent is still a *chord* — a sustained harmonic cushion — not a single thin
note. The recession thins the *attack density* (and displaces the surviving
attack off the downbeat), which is exactly the right perceptual lever (attention
is event-driven; fewer attacks = less figure-competition), but it does **not**
hollow the *harmonic* bed the way "+1.0 margin, bed at 1 onset" sounds like it
might on paper. **This is good news for the hollowing worry and it is the single
most important thing for the operator to confirm by ear** (knob 2). I size every
knob below with this metric-vs-percept gap in mind.

---

## 1. KNOB — `MIN_FIGURE_GAP` (seat guard) — RECOMMEND: keep at 2, but ear-gate it; lean to **1** if any leap is heard

**Recommendation: KEEP = 2 semitones as the A/B anchor; if the operator hears a
single jarring octave leap on a dark counter-routed step, drop to 1. Do not raise
above 2.** It only fires on the 3 counter-present images (`counter_present` gate,
`:1554`) and only when a dark-image brightness drop pulls the raw seat below
`COUNTER_CEILING(67) + GAP`.

**The perceptual mechanism behind the producer's flag is real and I confirm it
from the code.** `seat_pc_in_register(pc, floor)` (`:1604`) lifts the melody's
*pitch class* up to the first instance at/above `floor`. The guard raises `floor`
from a dark value (~55–57) to `67 + GAP`. With GAP = 2 the floor is **69**. The
octave-leap risk is precise: a melody whose top-chord pitch class lands **below**
the floor's pitch class gets seated a full register higher. Worst case: pc just
under 69 → seated at the next octave (e.g. realized 67 with GAP=0 vs **79** with
the lift) — the producer's 67→79 example. The size of the *worst* leap is roughly
independent of whether GAP is 1 or 2 (both land in the [67, 79] octave); GAP only
shifts *which pitch classes* trip the next-octave boundary. So GAP = 1 vs 2 is not
"smaller leap," it is "fewer pitch classes that leap." That argues mildly for
**1**: a tie-plus-one-semitone still satisfies high-voice superiority (a 1-st
gap is audibly "above," Bregman frequency-streaming does not need 2), while
shrinking the set of dark-step pitch classes that get kicked an octave up.

**Why keep 2 as the anchor anyway:** the guard fires only on dark, counter-routed
steps (a small fraction of the render), and a *clear* seat (2 st) reads more
unambiguously "on top" than a 1-st tie-break — register is cue rank 3, and a 1-st
gap is near the just-noticeable edge for a timbre-flat synth. The honest position:
**the trade is a guaranteed-clear-seat (correctness, every dark step) against a
rare audible octave jump (a taste cost on a subset of dark steps).** The
correctness side is the figure-ground invariant the whole slice exists to
guarantee, so it wins by default — but the leap is an ear-cost the operator must
adjudicate, because a melody that *lurches* up an octave to assert the seat is
itself a figure-ground defect (the figure should arrive, not jump).

**LISTEN FOR:** on AudioHaxImg1/2/3 specifically, on the *darkest* passage —
does the melody ever **leap up by an octave to a thin high note** in a way that
sounds mechanical / like the tune suddenly jumped registers? If yes → set GAP = 1
and re-listen; if the leap persists at 1, the seat guard is correct but the
*dark-image octave drop itself* (`lift = −12`, `:1535`) is the real culprit and
that is a separate (deferred) inverse-register concern, not this knob.

---

## 2. KNOB — Deep-tier Pad recession depth (`PAD_DEEP_RECESSION_CEILING = 0.40`) — RECOMMEND: **keep the strong deep recession**; verify-by-ear it does not hollow

**Recommendation: KEEP the strong deep recession (`PAD_DEEP_RECESSION_CEILING =
0.40`, deep Pad weight 0.30 → cap = melody_min − 1).** Cue-strength theory backs
it directly: onset-rate / rhythm-grid is the #1 segregation cue
(`design-s46-affect.md` §1), so a strong activity gap is the *most* perceptually
powerful figure-maker available in a timbre-flat texture. A subject image WANTS a
vivid, forward, unambiguous figure (the arousal→sharper-separation principle,
`design-s46-affect.md` §5) — the strong deep recession is the correct affect for a
clear-subject image.

**The hollowing worry is the right worry but is mitigated by the §0 finding.** A
deep-tier Pad capped to `melody_min − 1` (Sustained melody → cap floored to
PAD_ONSET_FLOOR = 1; Oblique/Subdividing melody → cap = 1) sheds *attack* events,
and for a BLOCK bed the surviving single stab is a still-sounding chord, not a
thin note (§0). And the floor (`PAD_ONSET_FLOOR = 1`, `:1154/1212`) makes silence
structurally impossible — the bed recedes, it never vanishes (the S43 "bed does
not vanish" floor, preserved). So the design is sound: **strong attack-recession,
harmonically intact, never silent.**

The residual risk is real but narrow: on a *calm, dark, subject* step the melody
floors to Oblique (2 onsets, via the activity floor) and the deep Pad caps to 1 —
that is a genuine 2-vs-1 attack texture, and if the Pad's single surviving event
is sparse-sounding (a short stab rather than a held cushion), the bed could read
**thin/hollow** rather than **receded**. This is precisely the metric-vs-percept
gap: F1 says "+1.0 margin, healthy lead"; the ear might say "the accompaniment got
hollowed." Hollowing, if it happens, will be heard on the **calmest** subject
passage, not the busiest.

**Do NOT soften pre-emptively.** Softening the deep recession to fix a *suspected*
hollow would trade away the strongest figure cue on exactly the images that most
want a clear figure. The correct move is: ship strong, and soften ONLY if the ear
confirms hollow. If softening is needed, the gentle lever is to nudge
`PAD_DEEP_RECESSION_CEILING` DOWN toward 0.30–0.35 (so the mid 0.40 tier reads
"cap-at-melody" instead of "cap-below"), not to weaken the floor.

**LISTEN FOR:** on AudioHaxImg1/2 (the two DEEP images), on the *calmest* section
— does the accompaniment sound **present and supportive (receded)** or **thin /
hollowed-out / like something dropped out**? Specifically: is there still an
audible harmonic cushion under the melody, or does the bed read as an occasional
isolated stab? Receded-but-present = keep; hollow = lower the deep ceiling and
re-listen.

---

## 3. KNOB — Recession-tier prominence weights (deep / mid / shallow) — RECOMMEND: **keep all three; well-spaced**

**Recommendation: KEEP as built.** The three tiers are perceptually
well-differentiated and correctly oriented for SUBJECT vs FIELD:

| Tier | Mel | Counter | Fill | Pad | melody−counter gap | reads as |
|---|---|---|---|---|---|---|
| DEEP `melody_lead_strong` | 0.90 | 0.45 | 0.30 | 0.30 | **0.45** | one clear figure, deep bed |
| MID `melody_forward` | 0.78 | 0.58 | 0.40 | 0.40 | **0.20** | forward melody, present bed |
| SHALLOW `melody_lead_gentle` | 0.72 | 0.65 | 0.45 | 0.45 | **0.07** | near-even texture, melody still on top |

The melody−counter prominence GAP (0.45 / 0.20 / 0.07) is the perceptually
load-bearing spacing, and it is monotone and well-scaled: a subject image gets a
**clear** lead, a field image gets an **even** texture where the melody only just
leads (the metric-rigidity caution from `design-s46-affect.md` §5 / aesthetics'
field-image warning, honored — the shallow tier deliberately does NOT force a
strong lead on an abstract image). Crucially the shallow counter (0.65) still sits
**below** the shallow melody (0.72), so the figure always wins the ordering even on
a field image — only the MARGIN bends, never the sign (the binding §5 constraint).
This is exactly the affect-conditioned-margin behavior the lens asked for.

Two notes, neither a change-request:
- The deep Fill/Pad at 0.30 are above the S43 0.25 recession floor — good, they
  recede hard but stay audible.
- DECISION-2 compliance confirmed: the MID counter is UNCHANGED at 0.58 (the
  0.58→0.55 DP-6 trim is correctly deferred to slice 3); the deep/shallow counter
  values (0.45/0.65) are the natural consequence of recession depth, not a
  back-doored MID trim.

**LISTEN FOR:** A/B a DEEP image (AudioHaxImg1) against a SHALLOW image (Lena or
magicstudio) — the DEEP one should sound **spotlit** (clear single figure over a
recessed bed), the SHALLOW one should sound **even / ensemble** (melody present
but not dominating). If both sound identically spotlit → the tier spacing is too
flat at the ear (unlikely given the 0.45-vs-0.07 gap, but it is the test). If the
SHALLOW image's melody **disappears** into the texture → the shallow tier went too
even; raise shallow Mel toward 0.74 or drop shallow Counter toward 0.60.

---

## 4. KNOB — Routing thresholds (DEEP ≥ 0.25, SHALLOW < 0.10) — RECOMMEND: keep DEEP gate; **flag AudioHaxImg3 as the perceptual mis-bin to listen for**

**Recommendation: KEEP the thresholds as the A/B anchor (DEEP ≥ 0.25, SHALLOW <
0.10, MID in between), but the operator should ear-judge AudioHaxImg3
specifically.** Mapping the live thresholds onto the pinned 6-image spread (§0
table):

- DEEP (≥ 0.25): AudioHaxImg1 (0.341), AudioHaxImg2 (0.284) — both clearly
  strong-subject. **Correct.**
- SHALLOW (< 0.10): magicstudio (0.084), Lena (0.052) — both genuinely
  low-contrast / even. **Correct.**
- MID (0.10–0.25): AudioHaxImg3 (0.203), example.jpg (0.136). **AudioHaxImg3 is
  the perceptual question.**

**The flag: AudioHaxImg3 is a structured, counter-ROUTED subject image (it is one
of the three `pad_bed_counter` images) but it lands in the MID prominence tier**
(0.203 < 0.25), so it gets the moving counter (a duet-rich texture) but only the
MID 0.20 melody-counter gap — not the DEEP 0.45 lead. Perceptually that is a
plausible tension: an image structured enough to earn the contrapuntal counter
line, but pitched at a recession depth that lets the counter sit relatively
forward (0.58 vs 0.78). On a subject image with an active counter, a *narrow*
melody-counter gap is the classic "two lines competing for figure" risk — exactly
the S45 trap this whole arc exists to resolve. The activity governor protects it
(the counter only moves when the melody moves), so it should not invert; but the
MARGIN may read thinner than the ear wants for a clearly-structured image.

This is a **threshold-placement** judgment, not a defect: lowering the DEEP gate
from 0.25 to ~0.20 would pull AudioHaxImg3 into the DEEP tier (clear lead over its
counter). The risk of doing so blindly is over-driving a lead on an image that may
genuinely want the richer duet — which is why it is an ear call, not a code change.

**LISTEN FOR:** on AudioHaxImg3 — can you still clearly track the melody as THE
figure over its moving counter, or do the two lines blur into one wandering
duet-texture (the S45 fusion alarm)? If it blurs → lower the DEEP gate to ~0.20 so
Img3 gets `melody_lead_strong` and re-listen. If the duet reads as *richness* (two
audibly distinct lines, melody clearly leading) → the MID bin is correct and the
governor is doing its job. example.jpg (0.136, no counter) is not at risk here —
it has no competing counter line, so its MID tier is uncontroversial.

---

## 5. KNOB — `PAD_WEAK_BEAT_FRAC = 0.5` (displaced Pad onset on the "and") — RECOMMEND: **keep 0.5**

**Recommendation: KEEP = 0.5.** Perceptually sound and arguably the cleanest
single choice. When a block bed is thinned to one surviving stab, leaving it at
offset 0 (the downbeat) would FUSE its onset with the Bass/Fill/Melody downbeat
attack — the S42 fusion signature (identical grids → voices merge), which kills
both the F5a anti-fusion metric AND the perceptual reading of the bed as a
*separate* accompaniment voice. Displacing the surviving stab to 0.5 (the "and"
of the beat) does two things at once that both serve the affect goal:

1. **Anti-fusion (cue rank 1):** the bed now occupies a distinct onset phase from
   the figure's downbeat, so it segregates into its own stream — it reads as a
   real comp figure, not a hollowed chord welded to the melody's attack.
2. **Keeps the strong beat clear for the figure:** the melody's downbeat attack is
   the figure's most salient event; clearing the bed off it lets the figure own
   the accent (the "bed off the melody's accent" steer).

0.5 specifically (the exact backbeat "and") is the most metrically stable off-beat
— it reads as a deliberate accompaniment comp (think the "and" of a ballad
left-hand), not as a random syncopation that would itself capture attention
(irregular onsets re-capture attention — `design-s46-affect.md` §2.3; we want the
*figure* to carry the syncopation, the *bed* to be predictable). A value like 0.75
would push the stab to the "a" of the beat — a more anticipatory, jazzier feel
that draws slightly MORE attention to the bed, which is mildly anti-recession. So
0.5 is the conservative, recession-correct anchor.

**LISTEN FOR:** on the DEEP images (AudioHaxImg1/2), does the receded bed read as
a **real, gentle off-beat accompaniment** (a comp you could tap to), or does it
sound like a **hollowed / disembodied chord** floating without a rhythmic role? If
it reads as a comp → 0.5 is right. If it sounds detached/aimless → try 0.75 for a
more grounded backbeat feel and re-listen (but expect it to pull a hair more
attention to the bed).

---

## 6. THE A/B WATCH-ITEM CHECKLIST (S46 §8 watch-items, made concrete to THIS build)

Render at `--seed 42`, with and without the change, on the named images. Listen
for, in priority order:

1. **Track + hum the melody as the SINGLE figure.** On AudioHaxImg1/2/3 (counter
   routed) — does the melody arrive as the attended line, or do melody + counter
   blur into one wandering texture? (Blur = fusion; the governor or the Img3 bin
   needs attention — knob 4.)
2. **Does the melody MOVE MOST?** On a calm subject passage — is the melody the
   busiest/most-moving line, with the counter audibly UNDERNEATH (still moving,
   but less)? (If a bed role sounds busier → F1 inverted at the ear; check the
   governor / activity floor, NOT level.)
3. **DARK image — does the melody hold figure WITHOUT leaping jarringly?** The
   combined seat-guard + dark-drop test (knob 1). The melody must stay on top
   *and* arrive smoothly — a jarring octave leap to assert the seat is itself a
   defect.
4. **Did the backgrounds recede WITHOUT hollowing?** On the calmest DEEP passage
   (knob 2) — is there still an audible harmonic cushion, or did the bed thin to
   isolated stabs? (Hollow → lower `PAD_DEEP_RECESSION_CEILING`.)
5. **Did the counter stay MOVING (S45 NOT forfeited)?** On an ACTIVE subject
   passage — when the melody subdivides, the counter should move with it (the
   inner texture is alive). If the inner voice went static → over-receded; the
   governor should let the counter move whenever the melody moves.
6. **Does the separation MARGIN audibly differ between FIELD and SUBJECT?** A/B
   AudioHaxImg1 (DEEP/spotlit) vs Lena (SHALLOW/even). If both sound identically
   separated → the affect-conditioning (the tier family) is flat at the ear (knob
   3); if Lena's melody vanishes → shallow tier too even.

The fusion alarms (any of these = stop and fix activity/register, NOT level):
melody and counter blur into one line (1); a bed sounds busier than the tune (2);
the bed welds to the melody's downbeat as a hollow chord (5 / knob 5).

---

## 7. AFFECT VERDICT

**Perceptually SOUND, with two ear-gated tuning watch-points — ship it to the A/B
and tune from the operator's ear.** The build invests in the right cues in the
right order: it makes the melody the most-ACTIVE line (governor + floor — cue rank
1, the strongest figure-maker) and structurally on-TOP (seat guard — cue rank 3),
image-conditions the MARGIN so a subject image gets a clear lead and a field image
keeps an even texture (the affect-conditioned separation that prevents a new
uniformity), and recedes the bed in ATTACK density off the figure's downbeat — all
while preserving S45's moving counter and never silencing the bed. It correctly
treats LEVEL as the deferred last-10%. This is the figure-ground hierarchy the S46
cadence specified, faithfully realized, with `engine.rs` byte-frozen throughout.
The two places it could be over- or under-done are both ear-decidable, not
structural: (a) the deep Pad recession could read hollow on the calmest subject
passage — mitigated by the §0 metric-vs-percept finding (a capped block bed is
still a chord), but the operator must confirm; and (b) AudioHaxImg3 sits in the
MID tier while being a counter-routed subject image, so its melody-counter margin
may read thinner than its structure wants — a threshold-placement ear call. Both
are starting-anchor tunings, not defects. The single largest risk is the knob-1
octave leap; the largest *quality* risk is the knob-2 hollow — both are first on
the listen-for list.

---

## RETURN TO THE LEAD (raw substance)

**Per-knob sizing:**
1. **`MIN_FIGURE_GAP`** — KEEP = 2 as anchor; drop to **1 if any jarring octave
   leap is heard** on a dark counter image. *Why: a clear seat is worth more than
   a rare leap, but a lurching melody is itself a figure defect — 1 st still
   satisfies high-voice superiority.*
2. **Deep Pad recession (`PAD_DEEP_RECESSION_CEILING = 0.40`)** — KEEP the strong
   recession; soften (toward 0.30–0.35) ONLY if the ear confirms hollow. *Why:
   onset-rate is the #1 figure cue, so a strong gap is correct for a subject
   image; and a capped BLOCK bed is still a chord, not a thin note (the F1 margin
   over-reads sparseness — metric counts NoteEvents, ear hears one attack).*
3. **Recession-tier weights (deep/mid/shallow)** — KEEP all three; well-spaced.
   *Why: the melody-counter gap 0.45 / 0.20 / 0.07 is monotone and correctly
   oriented; shallow counter (0.65) still sits under shallow melody (0.72) so the
   figure always wins the ordering, only the margin bends.*
4. **Routing thresholds (DEEP ≥ 0.25, SHALLOW < 0.10)** — KEEP, but **ear-judge
   AudioHaxImg3**. *Why: Img3 (ct 0.203) is a counter-ROUTED subject image that
   lands in the MID tier — moving counter at only a 0.20 melody-counter gap = the
   thinnest-margin competition risk on the set. If it blurs, lower the DEEP gate
   to ~0.20.*
5. **`PAD_WEAK_BEAT_FRAC = 0.5`** — KEEP. *Why: 0.5 (the "and") is the most
   metrically stable off-beat — it de-fuses the bed from the figure's downbeat and
   reads as a deliberate comp without itself capturing attention.*

**Single biggest perceptual RISK to listen for:** on a DARK counter-routed image
(AudioHaxImg1/2/3), the seat guard can kick the melody a full OCTAVE up (67→79) to
assert its seat — a melody that *leaps* to be the figure instead of *arriving* as
it. (Runner-up quality risk: the strong deep Pad recession reading HOLLOW rather
than RECEDED on the calmest subject passage.)

**A/B watch-item checklist:** (1) hum the melody as the single figure (no
melody/counter blur); (2) melody moves MOST, counter audibly under; (3) dark image
— melody holds top WITHOUT jarring leap; (4) bed recedes WITHOUT hollowing; (5)
counter still MOVES when the melody does (S45 intact); (6) separation MARGIN
audibly differs FIELD (Lena, even) vs SUBJECT (Img1, spotlit).

**Overall affect verdict:** SOUND — ship to the A/B as-is. Needs-tuning-where, all
ear-gated: knob 1 (drop to 1 if leap heard) and knob 4 (Img3 may want the DEEP
tier). Knob 2 is a verify-by-ear-don't-pre-soften. The build invests in the
strong cues (activity, register) in the right order, image-conditions the margin,
preserves S45, and keeps `engine.rs` frozen.

---

*End of S47 affect review. Review/design-only: no source, test, or asset
modified. `src/engine.rs` sha256 re-verified UNCHANGED:
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*

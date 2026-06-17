# Input S27 — K2b ResolutionPolicy: which forms resolve home vs may end open (Music Theory lane)

**Author role:** Music Theory Specialist — S27/K2b INPUT DOC. **DOC ONLY:** this file modifies no
source, test, or asset and owns no `.rs` file. It pins, per form, the per-scheme `ResolutionPolicy`
(`Resolve` | `Open`) that the K2b key-scheme catalogue rows will carry, gives the trained-ear
harmonic/formal judgment that backs each policy, names the ONE scheme endorsed as the first `Open`
showcase, and states the CONDITIONAL `resolves_home` test semantics precisely enough for the test
engineer to encode. **Date:** 2026-06-16. **HEAD:** `336c66a` (K2a — per-region affect + generalized
multi-excursion key plan + harmony re-root — BUILT & CLOSED).

**Builds on (do not restate):**
- `docs/input-s26-multiexcursion-harmony.md` §4 (when an open/off-home ending is COHERENT; the
  one-line criterion §4.1; the three coherent open-ending types §4.2; the broken cases §4.3; the
  forms/affect → ending map §4.4). This doc applies that §4 judgment to the concrete K2b catalogue.
- `docs/design-s26-multiexcursion-keyplan-engine.md` §2.1 (the `ResolutionPolicy` enum: `Resolve`
  forces the FINAL section's offset to 0; `Open` keeps it off-home), §2.4 (the catalogue rows under
  Option A; `theme_and_variations_excursion` proposed as `resolution: "open"`), §6 K2b (this slice),
  §7 Risk 2 (`resolves_home` becomes CONDITIONAL on policy; `open_schemes_may_end_off_home` is the
  new sibling test).

**Ground truth verified against the working tree at `336c66a` (not trusted from prose):**

- `assets/mappings.json` `form_catalogue` (`:90–119`) — the actual section roles + `boundary_cadence`
  per form, confirmed: `rounded_binary` ends **A'/Return/Perfect**; `ternary_aba` ends
  **A/Return/Perfect**; `aaba` ends **A/Return/Perfect**; `abac` ends **C/Coda/Perfect**; `abbac`
  ends **C/Coda/Plagal**; `theme_and_variations` is **T/V1/V2**, ending **V2/Development/Perfect**.
  (Note: the live T&V is a three-section T-V1-V2, not the four-section T-V1-V2-V3 sketched in design
  §2.4 — the policy judgment below is the same either way; the final section is a variation.)
- `key_scheme_catalogue` (`:166–177`) currently holds only `home_only`, `aba_excursion`, and
  `abac_rondo`; K2b adds the per-form excursion rows for the whole vocabulary. None today carries a
  `resolution`/`pivot` field, so all parse as the `Resolve`/`pivot:false` default (design §2.1) —
  i.e. today every scheme already ends home. K2b is therefore additive: it spells the policy
  explicitly per new row and introduces exactly one `Open` row.
- The realizer transpose seam (`chord_engine.rs:2104–2106`, READ-ONLY) consumes the per-section
  `key_offset_semitones` exactly once: `tonic_pc = (home_root_midi + ctx.section.key_offset_semitones)
  .rem_euclid(12)` for the theme melody tonic. There is **no pivot insert and no land-home cadence
  machinery in the file today** — that is K3. The load-bearing consequence for THIS doc: under K2b a
  `Resolve` scheme forces the final offset to 0, so the piece simply ends home as K1 did (smooth,
  already-proven); an `Open` scheme would leave the final offset non-zero and the piece would arrive
  in that off-home key **ABRUPTLY** — no prepared pivot, no cadence in the new key — because none of
  the K3 graceful-landing machinery fires yet.

---

## 0. Executive summary (read first)

The K2b policy assignment is **conservative and almost entirely `Resolve`**. Every returning form
(rounded binary, ternary, AABA) and every Coda/Return-ending form whose final boundary is a
Perfect or Plagal cadence MUST land home: the return/coda IS the homecoming, and ending off-home
would betray the form's own contract. The episodic ABAC traditionally feels less resolved, but per
the operator's locked Option A its C still resolves HOME under the **default** scheme; `Open` is a
separate sibling, not the default. **Exactly one form is endorsed as the first `Open` scheme:
`theme_and_variations_excursion`** — a variation set is the one idiom where wandering and an open
ending are stylistically native. Even it ships **OFF by default** (routed away in the `key_scheme`
SelectTable), because under K2b (no K3 pivot/cadence) an `Open` ending arrives abruptly; it is to be
judged by ear **only after K3** makes the landing graceful.

This matches §2.4 of the design exactly. I have **one refinement, not a disagreement** (§4): even
the `Open` T&V row should ship behind a SelectTable that does not route to it by default, and the
abbac/abac `Open` siblings the design floats as possibilities should NOT be authored in K2b — one
`Open` showcase is enough to prove the policy end-to-end, and more open rows before K3 only multiply
abrupt endings.

---

## 1. THE PER-FORM POLICY TABLE

Policy is grounded in two things for each form: (a) the form's NATURE (returning vs episodic vs
variation), and (b) its FINAL section's role + `boundary_cadence` as it actually exists in
`form_catalogue` at `336c66a`. A Perfect/Plagal cadence on a Return/Coda is a structural promise of
arrival; honoring the form means landing home there.

| Form | Final section (role / cadence) | **ResolutionPolicy** | Harmonic / formal rationale |
|---|---|---|---|
| **rounded_binary** | A' / Return / **Perfect** | **Resolve** | A rounded binary IS the depart-and-return promise (A–B–A'); the A' return on a Perfect cadence is the homecoming. Ending off-home would leave the returning A' stranded in a foreign key — the form contradicting itself. |
| **ternary_aba** | A / Return / **Perfect** | **Resolve** | The most explicit returning form: the recapitulated A in the home key, closed by a PAC, is the entire point. An open ending here is not a color, it is a broken return. |
| **aaba** | A / Return / **Perfect** | **Resolve** | The 32-bar song form: the final A after the bridge resolves the bridge's departure with a PAC. The bridge (B) is the one excursion; the last A is the answer. It must land home or the song "never comes back." |
| **abac** | C / Coda / **Perfect** | **Resolve** | Episodic forms feel *less* resolved than returning ones, and ABAC has no literal final A — but its C is a Coda that cadences Perfect, and per Option A the Coda resolves to HOME (Invariant A: new material, home key). Default policy lands home; an off-home `abac_open` sibling is a *separate*, not-default option, deferred to K3. |
| **abbac** | C / Coda / **Plagal** | **Resolve** | Two contrasts (B, B′) then a Return and a Plagal-cadenced Coda. The Plagal "amen" close IS a resolution gesture, and it resolves in the HOME key (Invariant A). A plagal home close is a legitimate, slightly softer homecoming — still a homecoming. Resolve. |
| **theme_and_variations** | V2 (live) / Development / **Perfect** | **Open** *(but routed OFF by default — see §2)* | The ONE form where the final section legitimately wanders and an open ending is idiomatic (the "dissolve / fade / final-variation-drifts-away" ending). A variation set is not a return-contract; each variation is a new lens on the theme, so ending in a related key reads as *the last variation living somewhere else*, not as a piece that failed to return. This is the natural first `Open` showcase — with the K2b caveat that it will sound abrupt until K3. |

**Reading the table against the form's own cadence data.** Every `Resolve` row has a final
Return/Coda carrying a Perfect or Plagal cadence in `form_catalogue` — the cadence label and the
policy agree: the realizer is told to close authentically/plagally, and `Resolve` guarantees that
close is in the HOME key (offset forced 0). The single `Open` row (T&V) also carries a Perfect
boundary_cadence on its final variation, which is exactly why an `Open` T&V is *coherent* rather than
broken: even off-home, the final variation still cadences (in its own key) — it STOPS on a stable
resting chord, satisfying the §4.1 coherence criterion of `input-s26`. It is not a piece that "ran
out"; it is a piece that ended its last variation in a neighboring key. (Caveat: that coherent-stop
property only becomes *audible* once K3 voices the cadence in the off-home key; under K2b the offset
moves but the cadence is not yet realized in the new key — hence ship-off-by-default, §2.)

---

## 2. THE ONE ENDORSED FIRST `Open` SCHEME

**Endorsed first `Open` scheme: `theme_and_variations_excursion`** (i.e. the key scheme routed onto
the `theme_and_variations` form), `resolution: "open"`, **`pivot: false`**, and — the
load-bearing operational caveat — **routed OFF by default in the `key_scheme` SelectTable** so no
image actually lands on it until the operator re-listens after K3.

**Why a variation set can end off-home without sounding broken.** Form is the argument here. A
returning form (binary/ternary/AABA) makes a *contract* with the listener: we will leave home and we
will come back; the return is the formal payoff, and withholding it reads as failure. A theme and
variations makes *no such contract*. Its logic is cumulative, not departure-and-return: the theme is
stated, then each variation re-illuminates it (rhythmically, texturally, harmonically). Ending the
final variation in a related key is heard as "this last variation simply lives over there" — the
material is still recognizably the theme, the relationship is still close (the K2b menu is
`{+7,+5,+3,−3}`, all closely-related), and the variation still cadences. It reads as a deliberate
expressive drift (the Romantic "dissolve" ending, the jazz "out-of-key final chorus that just stays"),
not as a piece that lost its way. This is precisely the `input-s26` §4.4 judgment that
`theme_and_variations` MAY end open, made concrete for the catalogue.

**The K2b caveat (why it ships OFF).** Under K2b there is NO K3 pivot/cadence machinery
(`chord_engine.rs` has no pivot insert and no land-home arming at `336c66a`). So an `Open` T&V will:
(a) leave the final variation's offset non-zero — correct, that is the feature; but (b) ARRIVE in
that off-home key abruptly, because the modulation into the final variation is an unprepared direct
move and the final cadence is not yet voiced in the off-home key. The result under K2b will sound
like a splice, not an intentional drift. **Therefore it is correct to ship the `Open` policy present
in the data (to prove the policy end-to-end and to land the conditional test, §3) but routed OFF by
default**, and to judge it by EAR only after K3 makes the pivot/landing graceful. The data carries
the deliberate-feature; the SelectTable withholds it from real images until it sounds intentional.
This is the operator's locked Option A: off-home open endings are a deliberate feature that ship OFF
by default, with T&V as the first (and in K2b, only) `Open` scheme.

---

## 3. THE CONDITIONAL `resolves_home` TEST SEMANTICS (for the test engineer)

The K1 `resolves_home` property asserted the FINAL section's resolved offset == 0 for EVERY form ×
scheme. Under K2b that is FALSE for any `Open` scheme (it legitimately ends off-home). The property
must split into two, keyed on the scheme's `ResolutionPolicy`. State both precisely:

### 3.1 `resolves_home` — the `Resolve` branch (strict homecoming)

> For every key scheme whose `resolution == Resolve` (including the `#[serde(default)]` case where
> the field is absent), run `resolve_key_scheme` for that scheme over its form's sections for an
> arbitrary firing `ImageUnderstanding`. **Assert: the LAST element of the returned `Vec<i8>` (the
> final section's resolved `key_offset_semitones`) == 0.** This is the strict homecoming guarantee:
> a `Resolve` scheme ALWAYS lands home regardless of the final section's own `offset_rule` (the
> policy forces the final offset to 0 — design §2.1 `Resolve`). This must hold for every `Resolve`
> row: `rounded_binary_excursion`, `ternary_aba_excursion`, `aaba_excursion`, `abac_rondo`,
> `abbac_excursion`, and the byte-stable `home_only`/`aba_excursion` identity/legacy schemes.

### 3.2 `open_schemes_may_end_off_home` — the `Open` branch (satisfied-or-coherent-open)

> For every key scheme whose `resolution == Open`, run `resolve_key_scheme` over its form's sections
> for a firing `ImageUnderstanding`. The final offset MAY be non-zero — that is the deliberate open
> ending — but it must still be a **legal menu offset**. **Assert: the final resolved offset ∈
> `{ +7, +5, +3, −3, 0 }`** (the v1 closely-related menu, OR 0). I.e. an `Open` scheme is allowed to
> END somewhere other than home, but it must end somewhere COHERENT — a closely-related key on the
> menu, never an arbitrary/garbage offset. (0 stays legal: an `Open` scheme is permitted to *happen*
> to resolve home for a given image — `Open` means "not FORCED home," not "forbidden from home.")
> A stronger optional assertion the test engineer MAY add for the deliberate-feature witness: assert
> there EXISTS at least one firing image for which an `Open` scheme's final offset is non-zero, to
> prove the open ending is reachable (design §7 Risk 2's "deliberate-feature witness").

### 3.3 The one-line contract for both

> **`Resolve` ⇒ final offset == 0 (strict).  `Open` ⇒ final offset ∈ {+7,+5,+3,−3,0} (coherent,
> may be off-home).**  Both branches share the existing `smooth_keys_only` floor (every resolved
> non-zero offset, in ANY position, is in `{+7,+5,+3,−3}`); the new split only governs what the
> FINAL position is allowed to be. No `Open` scheme may produce a final offset outside the menu —
> that is the line between "deliberate open" and "garbage key," and it is the entire safety content
> of the `Open` branch under K2b (where no realizer cadence yet validates the landing by ear).

---

## 4. CORRECTIONS / REFINEMENTS TO THE §2.4 POLICY ASSIGNMENTS

I **concur with §2.4's policy assignments**: every form `Resolve` except `theme_and_variations`,
which is `Open`. My ear does not disagree with a single assignment. Two refinements (not reversals),
both in service of the operator's "ships OFF by default" lock:

1. **Ship only ONE `Open` row in K2b (T&V), and do NOT author the floated `abac_open` /
   open-abbac siblings yet.** Design §2.4 mentions an optional `abac_open` sibling and §4.4 of
   `input-s26` notes abac/abbac MAY end open for low-arousal profiles. Those are real future colors,
   but authoring them in K2b would multiply abrupt (pre-K3) open endings for no test benefit — one
   `Open` scheme already exercises the conditional `resolves_home` split fully. **Stage abac_open /
   abbac_open as a post-K3 slice**, once the landing is graceful and they can be judged by ear.
   This keeps K2b's `Open` surface to exactly the form where openness is most idiomatic.

2. **The `Open` T&V row must be present in the catalogue but UNREACHABLE via the default
   `key_scheme` SelectTable** (the design says "ship `theme_and_variations` with `resolution: open`
   to prove the policy end-to-end"; I am pinning that the proof is via the DATA + the test, not via
   routing real images to it). Concretely: the `key_scheme` SelectTable should NOT add a rule that
   routes the `theme_and_variations` form onto the `Open` scheme by default; the open scheme exists
   for the test and for an explicit future opt-in. This is exactly the operator's Option A
   ("OFF by default, re-listen after K3") expressed as a routing constraint, and it is byte-safe:
   with no routing rule firing, `home_only` (or a `Resolve` T&V scheme) stays the selected default
   and no golden moves. (Whether a `Resolve` T&V scheme is *also* authored as the routed default for
   that form is an Aesthetics-lane SelectTable call; from the theory side a `Resolve` T&V is
   perfectly legitimate — a variation set that DOES end home is the classic finale-variation close.)

**One realizer-dependency flag (assigned to K3, NOT smuggled into K2b):** the `Open` ending only
*sounds* intentional once the final variation cadences in its off-home key with a prepared approach.
That requires the K3 pivot/land-home machinery in `chord_engine.rs`. Under K2b the `Open` policy is
data-correct and test-correct but not yet ear-correct. **Nothing in this doc requires a realizer
change for K2b** — the policy is forced/kept entirely in the planner's `resolve_key_scheme`, which is
already in scope for the slice. The graceful landing is K3's job; this doc only authorizes the
*policy bit*, not the *cadence realization*.

---

## 5. SUMMARY FOR THE BUILD ROLES

- **Implementer (sole committer of `assets/mappings.json`):** carry each K2b catalogue row with the
  policy in the §1 table — `Resolve` for `rounded_binary_excursion`, `ternary_aba_excursion`,
  `aaba_excursion`, `abac_rondo`, `abbac_excursion`; `Open` for `theme_and_variations_excursion`.
  Do NOT add a `key_scheme` SelectTable rule that routes real images onto the `Open` T&V scheme
  (§4 refinement 2). Do not author `abac_open`/`abbac_open` yet (§4 refinement 1). `pivot: false`
  on every K2b row (byte-safe; K3 flips it).
- **Test engineer:** encode `resolves_home` as the `Resolve` branch in §3.1 (final offset == 0) and
  `open_schemes_may_end_off_home` as the `Open` branch in §3.2 (final offset ∈ `{+7,+5,+3,−3,0}`,
  with the optional non-zero-reachability witness). The shared `smooth_keys_only` floor is unchanged.
- **Aesthetics lane:** owns whether a `Resolve` T&V scheme is authored as the routed default for the
  `theme_and_variations` form, and owns the broader SelectTable routing — within the §4 constraint
  that the `Open` scheme is not routed by default.

---

*Doc-only. No source, test, or asset modified. The per-form `ResolutionPolicy` assignments (§1), the
endorsed first `Open` scheme + its ship-off-by-default caveat (§2), the conditional `resolves_home` /
`open_schemes_may_end_off_home` semantics (§3), and the two §2.4 refinements (§4) are binding
harmonic/formal input for the implementer + test engineer to transcribe; the implementer is the sole
committer of `assets/mappings.json`. Realizer fact (no pivot/cadence machinery; single offset
consumption at the theme-tonic seam) verified against `chord_engine.rs:2104–2106`; the form roles +
boundary cadences verified against `assets/mappings.json` `form_catalogue`/`key_scheme_catalogue` at
HEAD `336c66a`. No realizer change is required by this doc (the graceful off-home landing is K3).*

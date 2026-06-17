# Spec S27 — K2b: the generalized key-scheme catalogue rows + the `key_scheme` routing rules

**Author role:** Composition & Songwriting Aesthetics Specialist — DESIGN/SPEC ONLY. This document
modifies no source, test, or asset. It produces exactly one artifact — this file. It specifies the
DATA that makes multi-excursion key plans REACHABLE: the full `key_scheme_catalogue` rows for every
form in the vocabulary, and the `key_scheme` `SelectTable` rules that route a real image onto each
scheme. The implementer is the SOLE committer of `assets/mappings.json`.

**Date:** 2026-06-16. **HEAD:** `336c66a` (AudioHax S27, BUILD K2b). **Verified against the working
tree at this HEAD** — every type, knob, offset-rule string, form section label/role, and the
role-alignment assertion referenced below was read in-tree, not trusted from prose.

**Builds on (do not restate):**
- `docs/design-s26-multiexcursion-keyplan-engine.md` — §2.4 (catalogue rows per form), §2.5 (the
  routing intent), §6 "Slice K2b", §7 (the conditional-`resolves_home` note / Risk 2).
- `docs/design-s26-multiexcursion-aesthetics.md` — the home-vs-open image condition (clear subject →
  home; subjectless panoramic + travelling read → may open).

---

## 0. What K2b fixes, in one paragraph

K2a is merged (verified): the per-region affect fields, the `ForegroundBrightness` /
`BackgroundBrightness` / `ForegroundHue` / `BackgroundHue` knobs, the `ResolutionPolicy` enum, the
`KeyScheme.{resolution, pivot}` fields, `parse_offset_rule` (`home` / `region_related:b|c|d`),
`region_excursion_offset`, and the generalized `resolve_key_scheme` (energy-DESCENDING region rank +
resolution-policy applied last) are all in `src/composition.rs` at this HEAD. But the SHIPPED DATA in
`assets/mappings.json` still only carries `home_only` / `aba_excursion` / `abac_rondo`, and the
`key_scheme` `SelectTable` routes ONLY `fg_bg_contrast ≥ 0.25 → aba_excursion`. So **`abac_rondo` is
DEAD DATA — no rule selects it** — and the whole-vocabulary travel the engine can now express is
unreachable. K2b is the byte-safe data slice that adds the remaining catalogue rows AND the routing
rules that fire them, so "the eye sweeps twice" actually happens. **K2b references DATA only**;
`chord_engine.rs` / `engine.rs` / any realizer is untouched (the realizer is byte-frozen until K3,
and with `pivot: false` on every row the realizer inserts NOTHING regardless).

---

## 1. GROUND TRUTH the rows must align to (verified in-tree at `336c66a`)

### 1.1 The REAL `form_catalogue` — section labels AND roles (this is the binding contract)

`resolve_key_scheme` aligns a scheme's `sections` to the chosen form's `sections` **by ORDER**, and
runs a debug-only role-alignment assertion (`src/composition.rs:1464–1474`):

> for every section index the scheme covers, `rule_is_home` MUST equal `role_is_home`, where
> `role_is_home := role ∈ {Statement, Return}` and `rule_is_home := parse_offset_rule(...) == Home`.

So a `home` rule MUST sit on a `Statement`/`Return` section, and a `region_related:*` rule MUST sit on
a `Contrast`/`Development`/`Coda` section. The rows below are built to satisfy this for the ACTUAL
catalogue (read from `assets/mappings.json`), which differs in two places from the prose in
`design-s26-multiexcursion-keyplan-engine.md` §2.4 — see §4 discrepancies.

| form id | section (label, role) list — VERBATIM from the tree |
|---|---|
| `rounded_binary` | `(A, Statement) (B, Contrast) (A', Return)` |
| `ternary_aba` | `(A, Statement) (B, Contrast) (A, Return)` |
| `aaba` | `(A, Statement) (A, Statement) (B, Contrast) (A, Return)` |
| `abac` | `(A, Statement) (B, Contrast) (A, Return) (C, Coda)` |
| `abbac` | `(A, Statement) (B, Contrast) (B', Contrast) (A, Return) (C, Coda)` |
| `theme_and_variations` | `(T, Statement) (V1, Development) (V2, Development)` — **3 sections, V1/V2 are `Development`** |

### 1.2 The REAL knobs a routing rule may read (`Knob` enum, `src/composition.rs:600–631`)

`edge_activity, texture, complexity, colorfulness, value_key, avg_brightness, avg_saturation,`
`dominant_hue, palette_bimodality, quadrant_contrast, vertical_emphasis, aspect_ratio, subject_size,`
`fg_bg_contrast, subject_energy, foreground_energy, background_energy, foreground_brightness,`
`background_brightness, foreground_hue, background_hue, arousal, valence`. The routing rules in §3
read ONLY `fg_bg_contrast`, `vertical_emphasis`, `quadrant_contrast`, `aspect_ratio`,
`palette_bimodality`, `complexity`, `edge_activity`, `value_key` — all confirmed real. (`subject_size`
is real but unused here; full home-vs-open subject gating is K3 — see §5.)

### 1.3 The REAL predicate JSON shape (`Predicate`, `src/composition.rs:677–688`)

`{"knob": <snake_case Knob>, "op": <lt|le|gt|ge|in_range>, "lo": <f32>, "hi": <f32>}` — `hi` used only
by `in_range`. `SelectTable` = `{"default": <id>, "rules": [{"when": [<Predicate>...], "pick": <id>}]}`;
ALL predicates in a `when` are AND'd; rules are first-match-wins (`SelectTable::select`,
`src/composition.rs:728–739`). The new rows below use only this shape.

### 1.4 The REAL `KeyScheme` fields + offset-rule grammar (verified)

`KeyScheme { id: String, sections: Vec<KeySchemeSection>, resolution: ResolutionPolicy (serde-default`
`Resolve), pivot: bool (serde-default false) }`; `ResolutionPolicy` JSON tags are `"resolve"` /
`"open"` (`#[serde(rename_all = "snake_case")]`). `KeySchemeSection { label: String, offset_rule:`
`String }`. `parse_offset_rule` accepts EXACTLY `"home"` → `Home`, `"region_related:b"` →
`Excursion(0)`, `"region_related:c"` → `Excursion(1)`, `"region_related:d"` → `Excursion(2)`; anything
else → `Home` (`src/composition.rs:1301–1308`). The two B's of `abbac` reading `region_related:b`
(rank 0) and `region_related:c` (rank 1) is what makes them DIVERGE (the "eye sweeps twice").

---

## 2. THE CATALOGUE ROWS — exact JSON to merge into `key_scheme_catalogue`

These rows are ADDITIVE to the existing array. Keep `home_only` and `aba_excursion` exactly as they
ship today (they rely on the serde defaults for `resolution`/`pivot` — byte-stable). The existing
`abac_rondo` row is REPLACED by the version below (it gains explicit `resolution` + `pivot`; its
`sections` are unchanged, so the resolved offsets are identical to today — the change is only that the
keys are now present and the row is reachable via §3).

Every row ships **`"pivot": false`** (K2b is byte-safe; K3 flips it). Every returning/home form ships
**`"resolution": "resolve"`**. ONLY `theme_and_variations_excursion` ships **`"resolution": "open"`** —
and per §3 it is routed OFF by default.

```jsonc
// ── ADD these rows to composition.key_scheme_catalogue (after the existing rows) ──

// rounded_binary → [A:Statement, B:Contrast, A':Return]  (3 sections)
{ "id": "rounded_binary_excursion",
  "resolution": "resolve",
  "pivot": false,
  "sections": [
    { "label": "A",  "offset_rule": "home" },              // Statement → home
    { "label": "B",  "offset_rule": "region_related:b" },  // Contrast  → rank-0 excursion
    { "label": "A'", "offset_rule": "home" } ] },          // Return    → home (resolve forces final 0)

// ternary_aba → [A:Statement, B:Contrast, A:Return]  (3 sections)
{ "id": "ternary_aba_excursion",
  "resolution": "resolve",
  "pivot": false,
  "sections": [
    { "label": "A", "offset_rule": "home" },               // Statement → home
    { "label": "B", "offset_rule": "region_related:b" },   // Contrast  → rank-0 excursion
    { "label": "A", "offset_rule": "home" } ] },           // Return    → home

// aaba → [A:Statement, A:Statement, B:Contrast, A:Return]  (4 sections)
{ "id": "aaba_excursion",
  "resolution": "resolve",
  "pivot": false,
  "sections": [
    { "label": "A", "offset_rule": "home" },               // Statement → home
    { "label": "A", "offset_rule": "home" },               // Statement → home
    { "label": "B", "offset_rule": "region_related:b" },   // Contrast  → rank-0 excursion (the bridge departs)
    { "label": "A", "offset_rule": "home" } ] },           // Return    → home

// abac → [A:Statement, B:Contrast, A:Return, C:Coda]  (4 sections) — REPLACES the existing abac_rondo row
{ "id": "abac_rondo",
  "resolution": "resolve",                                  // Invariant A: C resolves HOME even on new material (K2b)
  "pivot": false,                                           // K2b ships pivot:false (byte-safe); K3 flips it on
  "sections": [
    { "label": "A", "offset_rule": "home" },               // Statement → home
    { "label": "B", "offset_rule": "region_related:b" },   // Contrast  → rank-0 excursion
    { "label": "A", "offset_rule": "home" },               // Return    → home
    { "label": "C", "offset_rule": "region_related:c" } ] },// Coda      → rank-1 excursion (DISTINCT from B; then resolved home)

// abbac → [A:Statement, B:Contrast, B':Contrast, A:Return, C:Coda]  (5 sections)
{ "id": "abbac_excursion",
  "resolution": "resolve",
  "pivot": false,
  "sections": [
    { "label": "A",  "offset_rule": "home" },              // Statement → home
    { "label": "B",  "offset_rule": "region_related:b" },  // Contrast  → rank-0 region
    { "label": "B'", "offset_rule": "region_related:c" },  // Contrast  → rank-1 region (the two B's DIVERGE)
    { "label": "A",  "offset_rule": "home" },              // Return    → home
    { "label": "C",  "offset_rule": "region_related:c" } ]},// Coda      → rank-1 excursion (then resolved home)

// theme_and_variations → [T:Statement, V1:Development, V2:Development]  (3 sections) — THE OPEN scheme
{ "id": "theme_and_variations_excursion",
  "resolution": "open",                                     // the natural showcase of the OPEN policy — a wandering variation set
  "pivot": false,
  "sections": [
    { "label": "T",  "offset_rule": "home" },              // Statement   → home
    { "label": "V1", "offset_rule": "region_related:b" },  // Development → rank-0 excursion
    { "label": "V2", "offset_rule": "region_related:c" } ]}// Development → rank-1 excursion; OPEN keeps final OFF-home
```

### 2.1 Resolved-offset behavior of each row (so the implementer/test engineer can witness it)

With `resolution: "resolve"`, `resolve_key_scheme` forces the FINAL section's offset to `0`
regardless of its rule. With `resolution: "open"`, the final section keeps its resolved excursion
offset. So on a FIRING image (`region_related:*` resolving to some menu value `m ∈ {+7,+5,+3,−3}`):

| scheme | per-section resolved offsets (intent) | final lands |
|---|---|---|
| `rounded_binary_excursion` | `[0, b, 0]` | home |
| `ternary_aba_excursion` | `[0, b, 0]` | home |
| `aaba_excursion` | `[0, 0, b, 0]` | home |
| `abac_rondo` | `[0, b, 0, 0]` (C's `region_related:c` rule present + rank-1, forced 0 by resolve) | home |
| `abbac_excursion` | `[0, b, c, 0, 0]` (B≠B' by rank; C forced 0 by resolve) | home |
| `theme_and_variations_excursion` | `[0, b, c]` (final V2 keeps `c`) | **OFF-home (open)** |

Note for `abac_rondo` / `abbac_excursion` under K2b: C's resolved offset is forced to 0 (ends home as
K1 did), but its `region_related:c` rule is now present and rank-1 — so the moment K3 flips
`pivot:true`, C becomes a real distinct mid-section excursion that journeys out and lands home, with
NO catalogue edit. This is the K3 forward-compat property baked in now.

---

## 3. THE `key_scheme` ROUTING RULES — exact JSON to REPLACE `composition.key_scheme`

First-match-wins, more-specific multi-excursion schemes BEFORE single-excursion, `home_only` stays the
default. The scheme-selection knobs are chosen to COINCIDE with the `form` ladder's selection knobs
(verified §1.1 form table vs the in-tree `form` `SelectTable`) so the chosen scheme's section
count/labels align with the form actually selected — this is what keeps the role-alignment assertion
quiet AND keeps "two-trip key plan on a two-trip form" true (design `design-s26-multiexcursion-aesthetics.md`
Rule FL-1 / FL-2). Every routing rule ALSO requires the K1 subject/ground gate `fg_bg_contrast ≥ 0.25`
so a flat, structureless image still gets `home_only` (no travel without a real stratification).

```jsonc
// ── REPLACE composition.key_scheme with this whole block ──
{
  "default": "home_only",
  "rules": [

    // 1. abbac_excursion — busy DARK field (the longest episodic sweep; the two B's diverge).
    //    Mirrors the form ladder's abbac trigger (edge_activity ≥ 0.7 & value_key ≥ 0.6) + the subject gate.
    { "when": [
        { "knob": "edge_activity",  "op": "ge", "lo": 0.7 },
        { "knob": "value_key",      "op": "ge", "lo": 0.6 },
        { "knob": "fg_bg_contrast", "op": "ge", "lo": 0.25 } ],
      "pick": "abbac_excursion" },

    // 2. abac_rondo — TALL / vertically-travelling read (the eye sweeps top-to-bottom: two stops).
    //    Mirrors the form ladder's abac trigger (vertical_emphasis ≥ 0.6) + the subject gate.
    { "when": [
        { "knob": "vertical_emphasis", "op": "ge", "lo": 0.6 },
        { "knob": "fg_bg_contrast",    "op": "ge", "lo": 0.25 } ],
      "pick": "abac_rondo" },

    // 3. ternary_aba_excursion — strong quadrant contrast (a scene split into contrasting halves).
    //    Mirrors the form ladder's ternary_aba trigger (quadrant_contrast ≥ 0.6) + the subject gate.
    { "when": [
        { "knob": "quadrant_contrast", "op": "ge", "lo": 0.6 },
        { "knob": "fg_bg_contrast",    "op": "ge", "lo": 0.25 } ],
      "pick": "ternary_aba_excursion" },

    // 4. aaba_excursion — wide, low-bimodality "songlike" field (one idea stated, departed, returned).
    //    Mirrors the form ladder's aaba trigger (aspect_ratio ≥ 1.6 & palette_bimodality ≤ 0.3) + the subject gate.
    { "when": [
        { "knob": "aspect_ratio",      "op": "ge", "lo": 1.6 },
        { "knob": "palette_bimodality","op": "le", "lo": 0.3 },
        { "knob": "fg_bg_contrast",    "op": "ge", "lo": 0.25 } ],
      "pick": "aaba_excursion" },

    // 5. rounded_binary_excursion — the DEFAULT returning form with a real subject (clear focal point:
    //    leave it, come back). The single-excursion catch-all; ordered LAST so any specific form wins first.
    //    Replaces the old "aba_excursion" route (rounded_binary's real sections are [A,B,A'] → this row
    //    aligns to them; aba_excursion's [A,B,A] also aligns but rounded_binary is the form-ladder default).
    { "when": [
        { "knob": "fg_bg_contrast", "op": "ge", "lo": 0.25 } ],
      "pick": "rounded_binary_excursion" }

    // ── theme_and_variations_excursion (the OPEN scheme) is INTENTIONALLY NOT ROUTED. ──
    // OPERATOR-LOCKED: open / off-home endings stay OFF by default until a re-listen turns them on.
    // The scheme SHIPS in the catalogue (§2) and is fully reachable by adding the commented rule
    // below, but no ACTIVE rule selects it, so no generated piece ends off-home in K2b.
    // To opt in after the re-listen, INSERT this rule (it would go FIRST, before rule 1, mirroring
    // the form ladder's theme_and_variations trigger complexity ≥ 0.66 & edge_activity ≥ 0.6):
    //   { "when": [
    //       { "knob": "complexity",     "op": "ge", "lo": 0.66 },
    //       { "knob": "edge_activity",  "op": "ge", "lo": 0.6 },
    //       { "knob": "fg_bg_contrast", "op": "ge", "lo": 0.25 } ],
    //     "pick": "theme_and_variations_excursion" }
  ]
}
```

### 3.1 The routing table at a glance (condition → scheme), and the OPEN scheme OFF

| order | image condition (AND'd) | scheme picked | ends |
|---|---|---|---|
| 1 | `edge_activity ≥ 0.7` AND `value_key ≥ 0.6` AND `fg_bg_contrast ≥ 0.25` | `abbac_excursion` | home |
| 2 | `vertical_emphasis ≥ 0.6` AND `fg_bg_contrast ≥ 0.25` | `abac_rondo` | home |
| 3 | `quadrant_contrast ≥ 0.6` AND `fg_bg_contrast ≥ 0.25` | `ternary_aba_excursion` | home |
| 4 | `aspect_ratio ≥ 1.6` AND `palette_bimodality ≤ 0.3` AND `fg_bg_contrast ≥ 0.25` | `aaba_excursion` | home |
| 5 | `fg_bg_contrast ≥ 0.25` (catch-all, last) | `rounded_binary_excursion` | home |
| — default — | nothing fires (flat / no subject stratification) | `home_only` | home (static) |
| **OFF** | **`theme_and_variations_excursion` — SHIPPED in catalogue, NOT routed by any active rule (operator-locked OFF)** | — | (open) |

### 3.2 Per-rule aesthetic rationale (image → why that tonal-travel shape)

1. **`abbac_excursion` ← busy dark high-edge field.** A dense, dark, high-activity image has the most
   visual "places" for the eye to land, so it earns the longest episodic sweep — two contrasting B
   excursions (rank-0 and rank-1 regions DIVERGE) that frame a homecoming, the richest journey the
   catalogue offers, matched to the richest image.
2. **`abac_rondo` ← tall / vertically-travelling read.** A tall image is read top-to-bottom; the eye
   genuinely travels and makes two stops, so a two-trip key plan (B excursion, home, C excursion, then
   home) dramatizes the vertical sweep as departure → return → second departure → return.
3. **`ternary_aba_excursion` ← strong quadrant contrast.** An image cleanly split into contrasting
   halves wants the textbook statement → contrast → return: ONE clear excursion into the "other half"
   and a clean homecoming, the most legible single trip.
4. **`aaba_excursion` ← wide, uniform (low-bimodality) field.** A wide, songlike image has one
   memorable idea; the bridge (the single B) is the one place that departs and the hook (A) returns —
   the pop-song shape where the familiar landing home IS the payoff.
5. **`rounded_binary_excursion` ← any clear subject (catch-all).** A scene with a focal point but no
   special travelling/episodic character gets the safe universal single trip: leave the subject, come
   back to it — the classic departure-and-return that always satisfies.
6. **`theme_and_variations_excursion` (OFF) ← single dense busy field, IF enabled.** A variation set
   that recolors one idea and drifts off-home is the one idiomatic place an OPEN ending feels
   intentional rather than abandoned — which is exactly why it is held OFF until the operator's ear
   confirms the open landing reads as deliberate, not broken.

---

## 4. REAL-KNOB / GRAMMAR / FORM DISCREPANCIES found against the design doc

All resolved in favor of the IN-TREE truth (the rows above already reflect the resolution):

1. **`theme_and_variations` has 3 sections, not 4.** `design-s26-multiexcursion-keyplan-engine.md`
   §2.4 showed `theme_and_variations_excursion` with `[T, V1, V2, V3]` (4 sections, V3
   `region_related:b`). The REAL form is `[T:Statement, V1:Development, V2:Development]` (3 sections).
   My row aligns 1:1 to the real 3 sections (`home`, `region_related:b`, `region_related:c`). A 4th
   `V3` row would zero-pad/truncate-mismatch the form and would have no Development section to bind to.
2. **`theme_and_variations` V1/V2 are role `Development`, not `Statement`.** This SATISFIES the
   role-alignment assertion for `region_related:*` rules (Development is a non-home role), so the
   `Open` scheme is assertion-clean.
3. **`abbac`'s third section is labelled `B'` (role `Contrast`), not `B`.** My `abbac_excursion` row
   uses label `B'` to match (labels are informational, not the match key — alignment is by ORDER — but
   matching the label avoids reader confusion and any future label-based debug aid).
4. **The form id is `abac`, the scheme id is `abac_rondo`.** Confirmed: the form-catalogue id is
   `abac`; the key-scheme id stays `abac_rondo` (the existing, now-reachable id). Routing rule 2 picks
   `abac_rondo` on the same `vertical_emphasis ≥ 0.6` trigger that selects the `abac` FORM, so form and
   scheme agree.
5. **No `aba_excursion`-style mismatch.** The old route `fg_bg_contrast ≥ 0.25 → aba_excursion` is
   replaced by rule 5 → `rounded_binary_excursion`, whose `[A, B, A']` sections align to the
   form-ladder DEFAULT form `rounded_binary` `[A, B, A']`. (`aba_excursion` stays in the catalogue,
   unrouted-but-harmless; it is the K1 row and remains assertion-clean against any `[A,B,A]` form. The
   implementer may leave it or drop it — leaving it is the lower-risk choice.)
6. **All knobs and offset_rule strings used are real** (verified §1.2 / §1.4). No invented knob, no
   invented offset rule, no predicate field outside `{knob, op, lo, hi}`.

---

## 5. WHAT THE OWNER WILL HEAR + tunable thresholds for the re-listen

**What changes audibly with K2b:** before K2b, only `aba_excursion` could ever fire (one single-trip
shape, gated on subject contrast). After K2b, the form the image already selects now carries its
MATCHING tonal travel — a tall image sweeps twice (abac), a busy dark field takes the long episodic
route (abbac, the two B's landing in DIFFERENT related keys), a split-contrast image makes one clean
trip (ternary), a wide songlike image departs once on its bridge (aaba), and everything else with a
real subject makes the safe single trip (rounded_binary). Every piece STILL LANDS HOME (every active
rule is a `resolve` scheme) and the harmony still travels only as far as K1/K2a already made it —
**K2b is breadth, not new realizer smoothness.** No piece ends off-home: the one open scheme is
shipped but routed OFF, per the operator lock.

**Tunable thresholds for the re-listen (all zero-golden-risk — they only change WHICH scheme fires,
never the identity/byte-frozen path, and they are mirrors of the existing `form`-ladder seeds):**

| threshold | seed | what raising/lowering does | risk |
|---|---|---|---|
| `fg_bg_contrast ≥ 0.25` (the subject gate on every rule) | 0.25 | raise → more images fall to `home_only` (more static); lower → more travel | none (byte-safe; gates travel on/off only) |
| `vertical_emphasis ≥ 0.6` (abac route) | 0.6 | how "tall" an image must read before it sweeps twice | none |
| `edge_activity ≥ 0.7` + `value_key ≥ 0.6` (abbac route) | 0.7 / 0.6 | how busy+dark before the longest episodic route | none |
| whether to TURN ON `theme_and_variations_excursion` | OFF | enabling routes the open ending — the operator's explicit post-re-listen decision | none to bytes; aesthetic only |

**The one flag for the ear:** the abbac route is the only place two distinct excursions (B≠B') are
audible in K2b; if on a re-listen the two B's still sound too similar, that is the per-region affect
sensitivity (K2a) to tune, NOT this data slice — flagged so it routes to the right lane.

---

## 6. MERGE NOTE (for the implementer — the SOLE committer)

- **Files changed:** `assets/mappings.json` ONLY. No source, no test, no realizer.
- **Keys changed inside `assets/mappings.json` (all under the `composition` object):**
  1. `composition.key_scheme_catalogue` — ADD the six rows in §2 (`rounded_binary_excursion`,
     `ternary_aba_excursion`, `aaba_excursion`, `abbac_excursion`,
     `theme_and_variations_excursion`) and REPLACE the existing `abac_rondo` row with the §2 version
     (gains explicit `resolution`/`pivot`; sections unchanged). Leave `home_only`, `aba_excursion`
     untouched.
  2. `composition.key_scheme` — REPLACE the whole `SelectTable` with the §3 block.
- **No other `mappings.json` key, and no other file, is touched.** `chord_engine.rs` / `engine.rs` /
  every realizer stays byte-frozen; with `pivot: false` on every row the realizer inserts nothing,
  and every active route is a `resolve` scheme so no piece ends off-home.
- **The implementer is the SOLE committer of `assets/mappings.json`** (single-writer discipline). This
  doc commits nothing.
- **Correctness guarantees of these rows:** they use ONLY real `Knob` variants (§1.2), the real
  `Predicate` shape (§1.3), the real `offset_rule` grammar (`home` / `region_related:b|c`, §1.4), and
  section counts/labels/roles that ALIGN 1:1 with each form's actual sections (§1.1) so the
  role-alignment debug assertion (`src/composition.rs:1464–1474`) stays quiet. The OPEN scheme is
  shipped but unrouted (operator-locked).
- **Test-engineer hand-off (out of K2b scope, flagged):** per `design-s26-multiexcursion-keyplan-engine.md`
  Risk 2, the K1 `resolves_home` property must be CONDITIONAL on `resolution` once an `Open` scheme
  ships in the catalogue — assert final-offset == 0 only for `resolution: "resolve"` schemes, and add
  an `open_scheme_may_end_off_home` witness for `theme_and_variations_excursion`. Because the Open
  scheme is routed OFF, no GENERATED piece exercises the open path in K2b, but the catalogue-round-trip
  test should still confirm the row parses with `resolution: "open"` and resolves a non-zero final
  offset when driven directly.

---

## 7. OPEN QUESTION FOR THE OPERATOR

After the re-listen of the K2b breadth (every form now travels, all landing home): do you want
`theme_and_variations_excursion` turned ON (route 0 in §3, currently commented OFF), so a single dense
busy field can legitimately END OPEN on a related key? It is the one place an unresolved ending reads
as idiomatic rather than broken — but per your lock it stays OFF until you hear the resolved-home
breadth first and explicitly opt in.

---

*Spec/design-only. No source, test, or asset modified by this document. The catalogue rows and routing
rules are binding DATA shapes for the implementer to transcribe into `assets/mappings.json`; the
implementer is the sole committer. All types, knobs, offset-rule strings, form section labels/roles,
the predicate JSON shape, and the role-alignment assertion are verified against the working tree at
HEAD `336c66a`.*

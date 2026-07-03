# QG Review — S64 STEP 2: Phrase-Period / Cadence-Scheme Spread

**Reviewer:** Quality Gate (independent re-derivation; read-only). **Date:** 2026-07-03.
**Scope reviewed:** `src/chord_engine.rs`, `src/composition.rs`, `src/mapping_loader.rs`,
`assets/mappings.json`; new tests `tests/phrase_scheme_s64.rs`, `tests/keydiag_s64.rs`; design doc
`docs/design-s64-phrase-period-spread.md`; diagnostic `docs/diagnostic-s64-fixed-rhythm-key.md`.

## VERDICT: **PASS-WITH-ISSUES**

Freeze intact, all load-bearing invariants independently re-derived and confirmed, BALANCE LAW held,
module boundaries respected. The catalogue is real. Two builder-flagged caveats (n=16 density edge;
cross-platform selection at the 0.2 boundary) are **acceptable as scoped** — neither is a functional
blocker and neither can move a golden or regress a shipped gate. One genuine defect found: a **factual
error in the shipped `mappings.json` `phrase_scheme._note`** (says `magicstudio→group_6`; the actual
rules, the runtime decode, and the design doc §3 all give `period_4`). Comment-only, non-blocking, but
should be corrected before push because it misdescribes corpus behavior for a future tuner.

---

## Check 1 — FREEZE (the load-bearing guard) — **PASS**

- `sha256(src/engine.rs)` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — **matches** the frozen anchor. `engine.rs` is not in `git diff --stat` (untouched).
- **Identity/legacy path is byte-identical, re-derived from source, not trusted:**
  - `plan_phrases` (chord_engine.rs:828) is now a pure delegate → `plan_phrases_scheme(chords, Period4)`.
    Same signature `engine.rs:431` calls; no signature changed.
  - `Period4::phrase_len_for` (chord_engine.rs) → `legacy_period_len(n)`, which is the pre-S64 rule
    extracted **verbatim** (the `PHRASE_LENGTHS.iter().rev()...find(len<=n && n%len==0 && n/len>=2)`
    or-else-divides or-else-`4` chain). I diffed the extracted `legacy_period_len` against the deleted
    inline block: identical logic.
  - The partition/cadence-stamping loop (chord_engine.rs:888–958) is **unchanged** — only the phrase
    length feeding it changed. So `Period4` reproduces pre-S64 output exactly.
  - `PhraseScheme::from_id("")`, `"period_4"`, `"garbage"`, `"PERIOD_4"`, `"sentence8"` all → `Period4`
    (confirmed in code and asserted by `default_and_unknown_ids_resolve_to_period4`).
  - `PhraseScheme::default() == Period4` (derived `#[default]` on the variant).
- **`engine_equivalence`: 9 passed / 0 failed** (ran isolated). No golden moved. The suite is
  structurally immune anyway (it builds `fixed_plan()` under `identity()`, never calls
  `plan_phrases`/`composition::plan`), but the freeze delegate also makes the *plan* path byte-stable.
- `period4_is_byte_identical_to_plan_phrases` asserts `plan_phrases == plan_phrases_scheme(Period4) ==
  plan_phrases_scheme(default())` across n∈{8,9,12,14,16} — independently confirms byte-identity incl.
  the n=16 case where legacy prefers the 8-step phrase.

## Check 2 — BALANCE LAW — **PASS**

Grep of the `+` lines of the diff for `rand|thread_rng|rng|hash|siphash|DefaultHasher|seed`: the only
matches are the two comment strings that literally say "never RNG" / "NO RNG". **No RNG, no
hash-synthesis introduced.** The pre-existing `thread_rng` lives solely in `pick_progression`
(chord_engine.rs:132) and was **not** extended by this change. Selection is a first-match-wins
`SelectTable` over `complexity`+`edge_activity` (authored menu + real feature seed), fully
deterministic per platform.

## Check 3 — THE CATALOGUE IS REAL — **PASS**

Cadence positions re-derived from the stamping loop (cadence stamped where `position_in_phrase ==
this_len-1 && this_len>=2`; last phrase absorbs the remainder). On an 8-step section:

| scheme | nominal len | partition | cadences | design-doc claim | test assert | verdict |
|---|---|---|---|---|---|---|
| Period4 | 4 | 4+4 | {3,7} | {3,7} | {3,7} | ✔ symmetric period |
| Phrase5 | 5 (n≥8) | 5+3 | {4,7} | {4,7} | {4,7} | ✔ genuine asymmetric period, interior HC walks 1 later |
| Group6 | 6 (n≥6) | 6+2 | {5,7} | {5,7} | {5,7} | ✔ broad long phrase + codetta |
| Sentence8 | 8 (n≥8) | 8 | {7} | {7} | {7} | ✔ single terminal cadence (density 1) |

Each is a distinct cadence *placement*, not a mechanical re-slice: Phrase5 is a real unequal
antecedent/consequent, Group6 a deferred single arrival + short tail, Sentence8 a single-goal drive.
**One naming nuance (not a defect):** the `Sentence8` layer realizes the *cadence scheme* of a
Schoenberg sentence (one arrival at 8) but does **not** stamp the internal 2+2+4 basic-idea/repetition
micro-structure — the interior steps are plain `PhraseStart`+`Interior`. This is honest for a layer
that only controls cadence periodicity; the "Schoenberg sentence (2+2+4)" label in the doc/comment
slightly overstates what the code realizes. Below each scheme's min span it correctly falls back to
`legacy_period_len`, keeping very short sections identical across schemes.

## Check 4 — CORPUS SPREAD is real — **PASS**

I ran `keydiag_s64 --nocapture` to read the **runtime decoded** features + per-section layouts (the
property test uses literal inputs; this is the independent runtime witness):

| image | complexity (decoded) | edge | scheme (runtime) | 8-step layout | cadences |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.005 | 0.301 | period_4 | (short: S.h/Sh/S..h) | legacy |
| AudioHaxImg2 | 0.015 | 0.509 | period_4 | `S..hS..P` | {3,7} |
| AudioHaxImg3 | 0.229 | 0.475 | **group_6** | `S....hSP` | {5,7} |
| example | 0.905 | 0.719 | **sentence_8** | `S......P` | {7} |
| Lena | 0.164 | 0.471 | **phrase_5** | `S...hS.P` | {4,7} |
| magicstudio | 1.000 | 0.106 | **period_4** | `S..hS..P` | {3,7} |

**All four schemes reached at runtime, no collapse.** Four distinct cadence signatures across the
corpus ({3,7},{4,7},{5,7},{7}). The `S..hS..P` skeleton (period_4 assignment) drops from 6/6 → **3/6**
(Img1, Img2, magic) and is no longer universal — the operator's transcribed cross-piece signature is
broken on 3 of 6 pieces. Runtime matches the design-doc §3 table exactly.

## Check 5 — PRESERVED INVARIANTS — **PASS**

- **Full suite: 604 passed / 0 failed** (`cargo test --release`).
- Isolated re-run confirms: `engine_equivalence` 9/9; `phrase_scheme_s64` 7/7; `interlock_h3_s63`
  1/1 (the S63 melody+comp+bass H3 6/6 interlock **still holds** under the new non-4 periods);
  `variety_s45` 5/5; `variety_scorecard_s45` 3/3 (F5a floor incl. Img1 0.518 razor-thin — HELD, and
  Img1 is deliberately kept on period_4 so its baseline is untouched); `s52_probe_identity`,
  `prominence_s43`, `figuration_s20`, `counterpoint_s30` all green. Not gamed — these are the
  pre-existing nets, unmodified, passing under the changed planner.
- **`clippy --all-targets -D clippy::correctness`: clean (exit 0).** The only clippy output is
  `doc_lazy_continuation` **style** warnings in `tests/saliency_s18.rs` — a pre-existing file NOT
  touched by this change and not in the `correctness` category.

## Check 6 — THE n=16 DENSITY-BOUND EDGE / F5a leak — **PASS (bounded, acceptable)**

Cadence counts re-derived by hand for every scheme over n=8..16:

- **Reachable range n=8..14: every non-legacy scheme count ≤ legacy count** (independently verified;
  `no_scheme_exceeds_legacy_cadence_count` asserts n=8..14). The F5a density bound holds everywhere the
  corpus can reach.
- **n=16 leak is real and slightly broader than the doc states.** At n=16 legacy prefers an 8-step
  phrase → 2 cadences; **both `Phrase5` (5+5+5+1 → cadences {4,9,14} = 3) AND `Group6` (6+6+4 →
  {5,11,15} = 3) exceed legacy's 2.** Design-doc §7 names only Phrase5; **Group6 also leaks at n=16**
  (minor doc incompleteness). Sentence8 at n=16 = 2 (= legacy), safe.
- **Is n=16 reachable? NO for the corpus.** `steps_per_section = 8 + round(edge_activity*8)`; n=16
  needs `edge ≥ 0.9375`, n=15 needs `edge ≥ 0.8125`; corpus max edge = 0.719 → **n ≤ 14**. Confirmed.
- **Latent-only risk:** a *future, non-corpus* image with `edge ≥ 0.9375` AND `complexity ∈ [0.1,0.6)`
  would select phrase_5/group_6 and hit n=16 with 3 cadences. But (a) F5a is a **test-time** ratchet
  measured only on the corpus, so no shipped gate leaks; (b) at runtime it is at worst **one** extra
  cadence ring on one section — a bounded quality risk, not a determinism/freeze break. **Judgment:
  acceptable for this slice; recommend §7 be corrected to name Group6 and the n=16 bound documented as
  a known latent edge** (or clamp `phrase_len_for` to prefer the density-safe scheme at n≥15 in a
  follow-up).

## Check 7 — MODULE BOUNDARIES — **PASS**

`git diff --stat`: only `assets/mappings.json`, `src/chord_engine.rs`, `src/composition.rs`,
`src/mapping_loader.rs` (Music Theory's lane) + the two new test files. **No edits to** `engine.rs`,
`image_*`, `midi_output`, `modem`, or `bin/*`. In-lane, clean.

---

## Cross-platform selection fragility (check 4 sub-question) — **ACCEPTABLE per-platform variety, NOT a correctness bug**

Runtime decoded margins to the nearest selector threshold (the 0.2 group_6/phrase_5 split):

- **Img3: complexity 0.229 vs 0.2 → margin 0.029** — *within* the ~3e-2 macOS-libm divergence band.
  macOS could decode ≲0.2 and route Img3 to **phrase_5** instead of group_6.
- **Lena: complexity 0.164 vs 0.2 → margin 0.036** — marginally outside the band; a 3.6e-2 divergence
  could push Lena to **group_6** instead of phrase_5.

**Judgment:** this is real runtime fragility, but it is **not a correctness bug** and **not a freeze
risk**, for three independent reasons:
1. It cannot move a golden or break identity — the identity/neutral path is complexity 0 → period_4,
   nowhere near 0.2. `engine_equivalence` and every identity witness are unaffected.
2. The only possible flip is **phrase_5 ↔ group_6**, and **both are cadence-density-neutral** (2
   cadences at every reachable n, verified in check 6). So a cross-platform flip **cannot** regress
   F5a differently or add a ring step — the property that actually matters is preserved on both OSes.
3. The audible goal is met on **both** platforms: period_4 stays confined to Img1/Img2/magic, and
   Img3+Lena still both get a non-legacy off-square scheme — merely possibly swapped between two
   density-equivalent structures, which is aesthetically immaterial to the anti-sameness objective.

The Test Engineer's decision to assert the selector with **literal** inputs (not decoded floats) is
correct and keeps the test platform-exact (the S62.1 lesson). No test fragility. The runtime fragility
is a benign per-platform variety difference. **No action required; worth a one-line note in the design
doc.**

---

## Issues (all non-blocking)

1. **`mappings.json` `phrase_scheme._note` factual error (fix recommended):** the note reads
   `magicstudio(1.000,0.106)→group_6`. The actual rule evaluation (rule 2, `complexity≥0.85`→period_4,
   ordered before the group_6 rule), the runtime decode (period_4, `S..hS..P`), and design-doc §3 all
   give **period_4**. Comment-only, but it misstates corpus behavior. (The RULES themselves are
   correct.)
2. **Design-doc §7 names only Phrase5 for the n=16 leak;** Group6 also produces 3 cadences at n=16.
   Recommend documenting both and stating the n≤14 reachability bound as the guard.
3. **`Sentence8` "Schoenberg sentence (2+2+4)" label** slightly overstates: the code realizes the
   sentence's cadence *placement* (single arrival), not its internal 2+2+4 idea-repetition. Honest at
   this layer's grain; a doc footnote would remove the overstatement.

## Evidence summary

- `sha256(engine.rs)` = `e50c7db…2348261` ✔ frozen.
- `engine_equivalence` 9/9; `phrase_scheme_s64` 7/7; `interlock_h3_s63` 1/1; full suite **604/0**.
- `clippy -D clippy::correctness` exit 0 (only pre-existing style warnings in untouched saliency_s18).
- No RNG/hash added; module boundaries in-lane; runtime corpus reaches all 4 schemes, S..hS..P 6/6→3/6.

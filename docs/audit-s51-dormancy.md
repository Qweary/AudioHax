# AudioHax S51 Dormancy Audit — Ranked Ledger

**Status:** READ-ONLY analysis. No source modified. This document is the sole S51 deliverable.
**Repo HEAD:** `df845cf35c97185b17bd97bc178fec5c1c284a30`
**`src/engine.rs` freeze:** sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — verified UNCHANGED. Any finding touching `engine.rs` is flagged NOT freeze-safe (do-not-edit) below.

## 1. Framing

At the S50 close the operator flagged a recurring "built-but-not-active / not-tied-in" pattern across AudioHax development and asked for a *systematic* sweep of everything else dormant — with an honest verdict if the codebase is otherwise clean. This audit merges two completed read-only sweeps into one ranked ledger:

- **Sweep A — code-structure / seam lens** (Rust Architect): traces declarations to their readers and finds where a built thing has no live consumer.
- **Sweep B — musical-axis lens** (Music Theory): traces each musical lever to whether it can *bite* on real images, finding axes that are wired but architecturally unreachable.

The two lenses overlap on three seeds (meter, the rhythm-cell axis, `foreground_energy`); those are deduped into single reconciled entries below, taking the more precise statement where the lenses refine each other. Every distinct finding is preserved. The highest-stakes file:line anchors were spot-verified against the working tree at HEAD and corrected where they had drifted. The natural follow-on build is **fix-direction-2**: the within-piece rhythmic-IDENTITY arc (activate meter + image-derived phrase-length / harmonic-rhythm + a per-image "rhythmic motto"); this audit ends with a reuse-vs-build bridge to that work.

## 2. Executive Verdict

**The codebase is otherwise clean; dormancy is NOT pervasive.** Genuine accidental-dead code is small and bin-local (two 1–2-line items in the modem encode/unpack bins; the library crate has essentially none). The real recurring pattern the operator sensed is three distinct things, not rot: (i) **legacy schema blocks** — two whole `mappings.json` sections (`instrument_section`, `fine_detail`, nine SelectTables/dicts total) loaded and parsed but never read, superseded by the `chord_engine` logic that now owns those decisions; (ii) the **macro rhythm layer** deliberately gated off — meter has an empty rule table and no consumer, and the rhythm-cell selector is unreachable on real photos because it sits behind a theme gate they never trip; (iii) **one user-facing dead surface** — three CLI subcommands that parse but print "not yet wired." Surrounding all of this is a body of *legitimate documented forward seams* (engine command vocabulary, counterpoint recognition predicates, test-only anchors) that are NOT defects, plus several stale comments that misdescribe code which is actually live. **Single highest-value action: wire-or-hide the dead CLI arms (F-CLI / N1)** — it is the only finding that is simultaneously user-facing, actively misleading, cheap, and freeze-safe.

## 3. Ranked Dormancy Ledger

Ranked by (severity × match to the operator's "built-but-not-tied-in" concern). Operator-concern-matching and actionable findings rank high; documented forward seams and cosmetic comments rank low. All line numbers verified/corrected against HEAD.

| Rank | ID | Title | What | Where (reconciled) | Why inert | Classification | Sev | Freeze-safe? | Cheapest path (recommendation) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **D-CLI** (was N1) | Dead CLI subcommands | `analyze`, `modem`, `tui` parse but route nowhere; `render` half-wired | Command enum `cli.rs:36-53`; dispatch `main.rs:487-513` — Play runs; Render runs only with `--wav` else prints "recognized but not yet wired" (`:495-499`); Analyze "not yet wired" (`:501-503`); Modem "use the dedicated modem bins" (`:505-507`); Tui "use the dedicated audiohax-tui bin" (`:509-511`). `cli.rs:46-52` doc-comment says Tui arm exists "for grammar completeness" | Functionality exists via separate bins/lib but the unified-CLI arms were never dispatched; actively misleads a user who types the obvious command | INTENDED-FUTURE (softest — user-facing) | Medium | Yes | **WIRE** Modem/Tui (`lib`/`tui::run` exist, cheap) **or** `#[command(hide=true)]` / drop the arms |
| 2 | **D-CELL** (S2 + F1) | Rhythm-cell axis dead on real images | `pick_rhythm_cell` is computed but never invoked for any real photo | def `composition.rs:1806`; called ONLY at `composition.rs:1490` inside the `else` of the theme gate `composition.rs:1478` (`theme_behaviour=="absent" \|\| !needs_theme` ⇒ `Vec::new()`, selector skipped). `theme_behaviour` set at `composition.rs:1409` from `mappings.json:161-164` (default `absent`; rule `complexity ge 0.4` ⇒ `fragment`). Per-archetype cell vocabulary inline at `chord_engine.rs:3241` (`rhythm_cells`), count at `:3331` | **Reconciled to F1's sharper mechanism (refines S2):** not merely "force-pinned" — *not even reached*. Real photos cluster complexity 0.005–0.23, never ≥ 0.4, so no theme exists ⇒ `pick_rhythm_cell` is never called on a real image. Doubly stranded: the S50 cell re-range was reverted (`composition.rs:1770-1792`) because lowering `CELL_COMPLEXITY_PROFILED` ≤ 0.4 collided with the same 0.4 theme gate and force-pinned cell 3. `review-s50.md`'s "edge ramp reachable for themed images [0.4,0.66)" is true but vacuous (real images ~never themed) | ACCIDENTAL-DEAD (architecture-gated) + MISDESCRIBING-DOC rider | **High** | Yes | **WIRE (fix-direction-2 core):** the real lever is the *theme gate at 0.4*, not the cell cuts. Recommend **(b) decouple `pick_rhythm_cell` from the theme path / run per-piece** = precisely fix-direction-2's "per-image rhythmic motto" (alt (a): lower the `theme_behaviour` complexity gate) |
| 3 | **D-METER** (S1 + F2) | Meter declared, parsed, stored, never consumed | Full Meter pipeline exists except the consumer; output is hard 4/4 everywhere | Empty rule table `mappings.json:211` (`"meter": {"default":"four4","rules":[]}`); selected/parsed at `composition.rs:1409` via `parse_meter` (`composition.rs:2232-2238`, maps four4/three4/six8/two4); enum doc `composition.rs:1231`; stored on plan; ONLY non-parser/non-test reference is the test assert `composition.rs:2480`. No code branches on the meter value | Schema rules empty AND no downstream reader — even a non-empty table would change nothing because nothing maps meter → bar grid → beat strength → onset placement | INTENDED-FUTURE (deferred to fix-direction-2; spec-s50 §2.D) | **High** for operator goal / **Low** bug-risk | Yes | **WIRE (fix-direction-2 follow-on):** build a new consumer chain meter → steps-per-bar → beat-strength → onset placement. Schema + enum + parser already exist to reuse — only the consumer is missing |
| 4 | **D-PALETTE** (F4) | `palette_bimodality` hard-pinned to constant; two gate terms can never bite | Producer always emits 0.0, so every `le 0.3` test is trivially true | Only producer `pure_analysis.rs:766` (`palette_bimodality: 0.0`); defaulted 0.0 in `composition.rs`. Consumed by two rules both `palette_bimodality le 0.3`: form `aaba` `mappings.json:156` and key_scheme `aaba_excursion` `mappings.json:256` | Producer never computes a real value ⇒ constant 0.0 ⇒ both terms always-true no-ops, silently widening those rules. `aaba` / `aaba_excursion` now turn purely on `aspect_ratio ge 1.6`; the single-palette discriminator they were designed around is inert | ACCIDENTAL-DEAD (stubbed producer) | Medium | Yes | **REMOVE** the two terms now (honest), **or** implement palette-histogram bimodality in `pure_analysis.rs` (modest) only if form variety is a target |
| 5 | **D-INSTSEC** (F5) | Entire `instrument_section` block loaded/parsed, never read (5 dead SelectTables) | `edge_density_to_rhythm`, `line_orientation_to_interval`, `contrast_to_articulation`, `color_shift_to_chord_extension`, `texture_to_modal_color` | `mappings.json:61-87`; struct + load `mapping_loader.rs:83-90,203,256-260`. Zero runtime readers (grep of all five + `.instrument_section` excl. loader = nothing; positive control `.saturation_to_harmonic_complexity` IS read at `chord_engine.rs:232`) | S1-era flat mappings superseded by `chord_engine` logic (band-ladder `realize_rhythm`, articulation curve, `roman_to_chord_complex`, `modal_interchange_trigger`). `line_orientation_to_interval` is doubly dead — no orientation field is computed in `pure_analysis.rs`. The data also **contradicts** live behavior (`edge_density_to_rhythm` low→HalfNotes vs the live band ladder) → comprehension trap | ACCIDENTAL-DEAD (superseded legacy schema) + MISDESCRIBING-DOC | Low musically / Medium comprehension hazard | Yes | **REMOVE** the block + loader struct (recommended), or stamp a LEGACY/UNREAD banner |
| 6 | **D-FINEDET** (F6) | Entire `fine_detail` block loaded, never read (4 dead mappings) | `pixel_y_position_to_pitch`, `pixel_brightness_to_velocity`, `local_jaggedness_to_chromaticism`, `shape_to_ostinato` (incl. PedalPoint / AscendingOstinato / DescendingOstinato) | `mappings.json:88-101`; struct + load `mapping_loader.rs:92-96,204,266-270`. Zero readers (grep all four + `.fine_detail` excl. loader = nothing; no `PedalPoint`/`AscendingOstinato` symbol anywhere in code) | Superseded S1-era; pitch/velocity/ostinato now flow via the motif/contour engine + chord realizer. Ostinato vocabulary is genuinely ABSENT from the engine — schema'd, never built | ACCIDENTAL-DEAD for pitch/velocity/jaggedness; INTENDED-FUTURE-never-built for `shape_to_ostinato` | Low (silent), but ostinato is a latent capability gap (pedal points / ostinati are a real expressive axis currently unavailable) | Yes | **REMOVE** the dead dicts; **FILE** ostinato (pedal points / ostinati) as a future build |
| 7 | **D-FE** (S3 + F3) | `foreground_energy` near-dead as a *selector* but LIVE elsewhere | Computed; one texture-routing term is a token floor, but a separate region-ranking use is genuinely live | Computed `pure_analysis.rs:734,780`. **Path 1 (near-dead):** texture SelectTable term `mappings.json:344` (`foreground_energy ge 0.015`) is a documented token floor — real images emit 0.003–0.039 and all clear it; the real discriminator is the adjacent `fg_bg_contrast ge 0.15` (`_comment:343`). **Path 2 (LIVE):** region-energy ranking key `composition.rs:2115` in `resolve_key_scheme` (+ `Knob::ForegroundEnergy` `composition.rs:832`) ranks fg/bg regions for excursion key offsets; fires when `key_scheme` has excursion sections (3 structured images clear `fg_bg_contrast ge 0.25`, `mappings.json:259`) | **Reconciled:** the dormancy claim holds ONLY for the texture-selector term; the axis is otherwise live. Already routed-around at S45 | TUNING-MISCALIBRATION (texture term only) | Low | Yes | **REMOVE** the vestigial `:344` token term, or leave (harmless). Do NOT touch the live Path-2 ranking use |
| 8 | **N3** | `recover_symbol_timing` docstring describes a loop the body does not implement | Docstring claims an "early-late" three-position steering loop; body is a grid search | docstring `modem.rs:1796-1803` ("early / on-time / late"); body `modem.rs:1847-1879` is a per-burst stride GRID SEARCH (inline comment `:1854` deliberately says the opposite) | Code is correct and live; only the docstring lies | MISDESCRIBING-COMMENT | Low-Medium | Yes | **REWRITE** the docstring on next touch |
| 9 | **N4** | `motif_rhythm` JSON block never deserialized | Per-archetype cells + `_index_cuts` data mirror | `mappings.json:165-178`; `CompositionMappings` (`mapping_loader.rs:110-161`) has NO `motif_rhythm` field; grep = zero consumers. Authoritative cells live inline in `chord_engine.rs` `MotifArchetype::rhythm_cells()` (`:3241`) | Intentional tunable data mirror of the authoritative inline Rust cells; its own `_note` (`:166`) is honest that it is inert | INTENDED-FUTURE / documented data mirror (borderline) | Low (drift risk) | Yes | **KEEP** as mirror, or remove the 14-line block to cut drift risk |
| 10 | **N2** | Modem "COMPILING STUBS — TODO Pass C" banners misdescribe complete code | Banners imply unfinished work over fully live code | `modem.rs:1497`, `:2004`; Pass-A/B/C narrative `:1295-1302`. Code is live: `render_sync_preamble` (`:1589-1616`), `detect_burst_start` (`:1618`), `recover_symbol_timing` (`:1808`), `RsRate::shard_config` (`:2038`); 10 passing realair tests | The "stub/TODO" language is stale; the modem sync + FEC APIs are NOT stubs | MISDESCRIBING-COMMENT | Low | Yes | **FIX** banners on next touch |
| 11 | **N9** | `encoding` field never read | `TileEntry.encoding: String` parsed, never used | `src/bin/unpack_tiled_payload.rs:27`; compiler "field `encoding` never read" | Near-intended manifest mirror but currently dead | ACCIDENTAL-DEAD (or manifest mirror) | Low | Yes | **REMOVE**, or `#[allow]` with a note if it mirrors the manifest |
| 12 | **N8** | `seq` counter genuinely accidental-dead | `let mut seq = 0usize` incremented, never read | `src/bin/modem_encode.rs:181` (declare), `:188` (increment); live `unused_variables` / `unused_assignments` warning | Pure accidental-dead — the only finding raising a non-pre-existing build warning | ACCIDENTAL-DEAD | Low | Yes | **REMOVE** (`:181` + `:188`) — clears a real warning |
| 13 | **N5** | `excursion_offset` test-only anchor | Function used only as a freeze-equivalence reference | `composition.rs:2067-2075` (`#[allow(dead_code)]`); production uses `region_excursion_offset` (`:2074,:2151`); `excursion_offset` called only from `#[cfg(test)]` `:2571`. Comment `:2062-2066` "keep as reference, do not delete" | Legitimate freeze-equivalence test anchor | INTENDED-FUTURE (test anchor) | Low | Yes | **LEAVE** |
| 14 | **N6** | `CounterFigure::Cambiata` + `is_legal_cambiata` recognition-only seam | Variant + predicate exist; never emitted | variant `chord_engine.rs:5176-5177` (`#[allow(dead_code)]`, "emission is a Slice-4 widening"); predicate `is_legal_cambiata` `:5276-5277` (`#[allow(dead_code)]`, "recognition-only") | Recognized but intentionally not yet emitted; matched documented pair | INTENDED-FUTURE (matched pair) | Low | Yes | **KEEP DEFERRED** |
| 15 | **N7** | `EngineCommand` 5/7 variants unused by any front-end | Forward control vocabulary for a future GUI/TUI | enum `engine.rs:227-243` (`SetInstruments`/`SetMsPerStep`/`SetThickness`/`Pause`/`Seek`); handled in `Engine::command` (`:631-652`); only `audiohax-tui.rs:97-98` sends Stop+Play. Doc `:225/:264` frames it as the GUI/TUI control vocabulary forward seam; handlers are real + unit-testable | Intentional forward seam; handlers exist and are tested | INTENDED-FUTURE (forward seam) | Low | **NO — `engine.rs` is FROZEN.** Do NOT propose edits to it | **KEEP** (do-not-edit; frozen) |

## 4. Category Summary

**ACCIDENTAL-DEAD → remove**
- D-CELL — rhythm-cell selector unreachable behind the 0.4 theme gate on real images (architecture-gated; HIGH — but its fix is fix-direction-2, not deletion).
- D-PALETTE — `palette_bimodality` pinned to 0.0; two `le 0.3` terms are silent no-ops.
- D-FE (texture term only) — `foreground_energy ge 0.015` is a token floor; Path-2 region ranking stays.
- N9 — `TileEntry.encoding` parsed, never read.
- N8 — `seq` counter incremented, never read (only real build warning).

**LEGACY-SCHEMA loaded-but-unread → remove / honesty cleanup**
- D-INSTSEC — entire `instrument_section` block (5 SelectTables) superseded by `chord_engine`.
- D-FINEDET — entire `fine_detail` block (4 mappings) superseded; ostinato vocabulary never built.
- N4 — `motif_rhythm` data mirror; never deserialized (borderline; honest `_note`).

**MACRO-RHYTHM gated-off → fix-direction-2 reuse**
- D-CELL — un-gate `pick_rhythm_cell` from the theme path (the "rhythmic motto").
- D-METER — build the meter → bar-grid → beat-strength → onset consumer chain.

**DOCUMENTED FORWARD SEAMS → keep**
- N5 — `excursion_offset` freeze-equivalence test anchor.
- N6 — `Cambiata` variant + `is_legal_cambiata` recognition-only pair.
- N7 — `EngineCommand` GUI/TUI control vocabulary (frozen `engine.rs` — do not edit).

**MISDESCRIBING COMMENTS → fix on next touch**
- N2 — modem "COMPILING STUBS — TODO Pass C" banners over live, tested code.
- N3 — `recover_symbol_timing` docstring describes an early-late loop; body is a grid search.
- (D-CELL and D-INSTSEC also carry a MISDESCRIBING-DOC rider, tracked under their primary class above.)

## 5. What Is *NOT* Dormant (verified-LIVE counter-findings)

The audit checked these candidate dormancy claims and **cleared them as LIVE** — the live machinery was confirmed, not merely assumed:

- **Melody foregrounding / prominence** — LIVE since S43 (`melody_forward` default `mappings.json:394`; escalation gate lowered 0.25 → 0.10; example ct 0.1355 escalates, Lena 0.052 takes default).
- **Harmonic complexity (triads / 7ths / 9ths)** — LIVE (`chord_engine.rs:231,348`, saturation-driven; real saturation 30–64).
- **Secondary dominants on the busy half** — LIVE (`chord_engine.rs:261-276`, `edge_activity > 0.55`).
- **Modal interchange (borrowed iv)** — LIVE (`chord_engine.rs:247-252,290`, `brightness_drop > 0.25`; Img1 bright 29.3 → drop ≈ 0.41).
- **Mode-mixture (bVI)** — LIVE (`chord_engine.rs:306-310`, `colorfulness > 0.45`).
- **NCT / species counterpoint** (passing / neighbor / suspension / cambiata-recognition) — LIVE, wired into the CounterMelody realizer (`chord_engine.rs:5234-5974`).
- **Character / tempo spread** — NOW LIVE at S50 (`mappings.json:204-208`, `arousal ge 0.34`).
- **Band ladder re-ranged cross-piece** — LIVE at S50 (`band_activity_spread` `chord_engine.rs:1097-1103`, wired `:2677,:1147`).

**Net:** the harmonic and per-step-rhythm machinery is largely live. Dormancy concentrates in (a) the macro / structural rhythm layer and (b) legacy schema blocks.

## 6. Fix-Direction-2 Reuse-vs-Build Appendix (bridge to S52)

Fix-direction-2 = the within-piece rhythmic-IDENTITY arc: activate meter, derive phrase-length / harmonic-rhythm from the image, and give each image a per-image "rhythmic motto."

**REUSE (declaration half already exists)**
- **Meter enum + table + parser** — only the consumer is missing (D-METER).
- **`pick_rhythm_cell` + the per-archetype cell vocabulary** (`chord_engine.rs:3241` `rhythm_cells`) — the "rhythmic motto" *is* `pick_rhythm_cell` liberated from the theme path. **Biggest reuse.**
- **`band_activity_spread` one-knee monotone spread mechanism** (`chord_engine.rs:1097`) — reusable for per-piece phrase-length / harmonic-rhythm curves.
- **SelectTable + catalogue + `Knob` infra** (`composition.rs:817-839`) — add new axes with zero new plumbing.

**BUILD NEW**
- The meter → bar-grid → beat-strength → onset-placement consumer chain.
- A per-piece phrase-length plan + per-piece harmonic-rhythm curve (currently fixed `{4,8}` + one policy).
- Image-varied pre-cadence acceleration (currently fires identically every phrase → the same "triplet + long-note" tail on every piece).

**Leverage insight:** fix-direction-2 is **more reuse than build.** Un-gating `pick_rhythm_cell` (D-CELL path b) delivers per-piece gait variety cheaply = the natural first move; the meter consumer (D-METER) is the structural follow-on. D-PALETTE / D-INSTSEC / D-FINEDET are pure cleanup that make `mappings.json` honestly reflect what the engine actually reads.

## 7. Suggested S52 Disposition Menu

These are bundles for **operator decision** — nothing here is pre-decided. All except Bundle 3's feature work are low-risk and freeze-safe.

**Bundle 1 — "Honesty cleanup"** (low-risk, freeze-safe; makes the tree honestly reflect itself)
- Remove the legacy schema blocks: D-INSTSEC (`instrument_section`), D-FINEDET (`fine_detail`), and optionally N4 (`motif_rhythm` mirror).
- Remove the accidental-dead bin items: N8 (`seq`), N9 (`encoding`).
- Remove the vestigial selector terms: D-FE texture token floor (`:344`), D-PALETTE's two `le 0.3` no-op terms.
- Fix the misdescribing comments: N2 (modem stub banners), N3 (`recover_symbol_timing` docstring).
- File the ostinato capability gap (from D-FINEDET) as a future build.

**Bundle 2 — "CLI surface"** (one user-facing fix)
- D-CLI: either wire Modem/Tui (the `lib`/`tui::run` targets exist) **or** `#[command(hide=true)]` / drop the unimplemented arms so the CLI stops misleading users.

**Bundle 3 — "Fix-direction-2 kickoff"** (the real feature work; in-class generative → run under a taste-gate cadence)
- Un-gate the rhythm-cell axis: D-CELL path (b) — decouple `pick_rhythm_cell` from the theme path / run per-piece.
- Build the meter consumer: D-METER — meter → bar-grid → beat-strength → onset placement.
- (Both are generative/aesthetic; acceptance turns on ear-judgment, so summon a taste/affect review voice into the build cadence.)

**Bundle 4 — "Keep deferred"** (legitimate forward seams; no action)
- N4 (if kept as mirror), N5 (`excursion_offset` test anchor), N6 (`Cambiata` recognition pair), N7 (`EngineCommand` vocabulary — **frozen `engine.rs`, do not edit**).

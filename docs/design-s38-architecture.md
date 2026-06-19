# Design S38 — Architecture & Freeze-Safety Diagnosis

**Author:** Rust Architect (design-only; no `src/*` edited this dispatch)
**Scope:** Independent architecture/feasibility diagnosis of three re-listen findings, with
exact change-surface, freeze-safety, data-flow, test-net impact, and risk/size per finding,
followed by a Slice-1 recommendation. Feeds the S38 synthesis dispatch.

## 0. Freeze boundary — verified

- `src/engine.rs` is BYTE-FROZEN at sha256 `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-hashed this dispatch: MATCHES).
- The frozen file's relevant entry points: `PipelineEngine::compose_from_image` (engine.rs:363) is a **thin wrapper** — it clones the `composition` mapping block, constructs `CompositionPlanner::new(...)`, calls `planner.plan(understanding, &self.mappings)` (composition.rs:1256), copies `plan.key_tempo.home_mode` onto `self.mode`, and installs via `set_plan`. **It reads NO per-image musical decision itself.** Therefore any planner-internal change to how `home_root_midi`, archetype, or density are *derived* is invisible to engine.rs — the wrapper passes through whatever `plan()` returns.
- The tick-time decision kernel `decide_instrument_action` (engine.rs:699) delegates to `chord_engine::realize_step` (engine.rs:740), passing a `&StepContext` (`ctx`). The kernel reads section data (incl. `ctx.section.density`) only *through* `ctx`; it never recomputes it. So changing the *data* on `ctx.section` does not edit the frozen kernel.
- `src/chord_engine.rs` is **NOT** in the freeze. `realize_rhythm`/`realize_step`/`resolve_motif`/`generate_chords`/`walking_bass`/`figured_bed` all live there and are editable — BUT they sit on the tick-time realize path called from the frozen kernel, so the byte-stability proof is **behavioral** (`engine_equivalence` goldens), not file-hash. The proof hinges on the identity preconditions (density==0.5, offset==0, MS_PER_STEP fixed) that the goldens' hand-built fixtures hold.

### Test-net inventory (what each fix must preserve)

| Test | What it pins | Calls planner? | Asserts planner's `home_root_midi`? |
|---|---|---|---|
| `engine_equivalence` (9/9) | `decide_instrument_action` byte-output against a **hand-built** plan: `home_root_midi:60`, `MS_PER_STEP=200`, `density:0.5`, `key_offset:0`. **Never calls `compose_from_image`/`plan()`** (test header §"Why a fixed plan") | No | No |
| `runtime_reachability_s37` | `compose_from_image` installs a plan; `total_steps==Σ section.step_len`; distinct images → distinct plans via **OR(tempo, mode, length)** (line 198–213) | Yes | No |
| `diversity_s13` | tempo/harmony/articulation divergence; uses `60` as a fixed `generate_chords` root arg | partial | No (60 is an input arg) |
| `composition_s15` | planner structural invariants; the `home_root_midi:60` at :444/:464 is a **hand-built `KeyTempoPlan` fixture** for `single_section_default`, not a planner-output assertion | Yes | No |
| `keyplan_s25/k2a/k2b/k3/s29` | re-root/offset/pivot math. `HOME:u8=60` (k2a:304) is a **local reference base** for *relative* re-root assertions (chord i at R+off == chord i at R transposed by off); `home_root_midi:60` elsewhere are hand-built fixtures | Yes (k2a/s25) | No |

**Key result:** Grep over all `tests/` for a test that both runs the planner and asserts `plan.key_tempo.home_root_midi == <const>` returns **zero hits**. Every `home_root_midi: 60` / `HOME = 60` is either a hand-built fixture (immune to planner changes) or a relative-math reference base (root-agnostic). This is the load-bearing fact for Finding 1's freeze-safety.

---

## Finding 1 — Every image starts on the same root

**Locus confirmed:** `composition.rs:1302` `let home_root_midi = 60;` (active compose path, inside `CompositionPlanner::plan`). The constant flows at composition.rs:1418–1419 into `section_root_midi = (home_root_midi as i16 + section_offset as i16).clamp(0,127) as u8`, then into `chord_engine.generate_chords(&progression, section_root_midi, ...)` (1425) and `tonic_triad(section_root_midi, ...)` (1441). It is also stored on `KeyTempoPlan.home_root_midi` (1484). The legacy `cli.rs:235 root_midi:60` / engine.rs:206 `EngineConfig` default feed the *separate* `set_features_global` legacy path (engine.rs:416 `self.config.root_midi`), which the pure compose path does NOT use.

**(a) Change surface — composition.rs ONLY, Rust + optional data.**
Replace the literal at composition.rs:1302 with an image-derived lookup. `u.dominant_hue` is **already** in scope and already read one line above (1294–1296, `lookup_range_map(&mappings.global.hue_to_mode, u.dominant_hue)`). Two viable shapes:
- **Data-driven (preferred):** add a `hue_to_root` (or `hue_to_root_offset`) range map to `assets/mappings.json` under `global` (sibling of the existing `hue_to_mode`, mappings.json:3) and reuse `mapping_loader::lookup_range_map` to resolve `u.dominant_hue` → a root MIDI (or a semitone offset added to a 60 base). ~3 Rust lines + one JSON block. No new loader type (`lookup_range_map` already exists and is already used here).
- **Inline Rust:** a small pure fn `fn home_root_from_hue(hue: f32) -> u8` in composition.rs mapping hue quadrants → roots. No JSON, but bakes policy into Rust (less tunable).

**No engine.rs touch. No main.rs touch.** The kickoff hypothesized a "possible small freeze-safe main.rs touch" — **it is NOT needed.** main.rs:288/306 (render) and 548/570 (play) call `understand_image_pure(...)` then `engine.compose_from_image(&understanding)`; `dominant_hue` is already populated inside `ImageUnderstanding` by `understand_image_pure` (pure_analysis.rs:737 `dominant_hue = g.avg_hue`, :757 field set). main.rs hands the whole `ImageUnderstanding` to the frozen wrapper unchanged; the derivation happens *inside* `plan()`. main.rs sees nothing.

**(b) Freeze-safety: SAFE — does NOT touch engine.rs.** The byte-freeze anchor is `home_root + 0 == home_root` for the *identity* path. Image-derived root changes the *value* of `home_root_midi`, but:
- `engine_equivalence` builds its plan by hand with `home_root_midi:60` and never calls `plan()` — it cannot observe a planner change. **9/9 stays green by construction.**
- The re-root seam math (keyplan_k2a `harmony_reroots`) asserts *relative* behavior (offset transposes every note by exactly `off`), which holds for ANY base root. Root-agnostic.
- **No design here wants to touch engine.rs.** If a *future* slice wanted root selection in the tick kernel (it should not — root is a plan-time spine decision), that would require operator sign-off before breaking the freeze. Flagging proactively: nothing in Finding 1 needs it.

**(c) Data-flow — already plumbed.** `dominant_hue` → `ImageUnderstanding` (pure_analysis.rs:639/737/757) → `plan(u, ...)` (1256), in scope at the 1302 site, already consumed at 1294. **Zero threading work.** (`value_key`/`avg_brightness` are also in scope if the design wants brightness to bias octave/register, but hue→pitch-class is the canonical and minimal choice.)

**(d) Backward-compat / test-net impact.** `engine_equivalence` 9/9: unaffected (hand-built plan). `runtime_reachability_s37`: STRENGTHENED — its `distinct_images` OR-clause (line 202) gains a fourth potential divergence axis; if a future tweak ever flattened mode AND tempo AND length, root would still distinguish, but the test does not currently read root so it neither requires nor forbids it. `composition_s15`/`keyplan_*`: the hand-built `home_root_midi:60` fixtures are immune; the *docstring comments* ("the planner's home_root_midi seed", k2a:304/289) become **stale and must be reworded** (cosmetic, but a Crosscheck-style consistency item). `diversity_s13`: the `60`-arg `generate_chords` calls are direct unit calls, untouched.

**(e) Risk + size.** Risk LOW. The only behavioral-stability subtlety: confirm the chosen root stays in a singable band so `generate_chords`/`voice_lead_sequence` don't clamp at the 0/127 MIDI rails (the existing `.clamp(0,127)` at 1419 already guards). Size: **S (≈1 build slice, <30 LOC + 1 JSON block + stale-comment sweep).** Smallest of the three.

---

## Finding 2 — Too few motifs

**Locus confirmed:** `pick_archetype()` (composition.rs:1500) returns only **4 of 8** `MotifArchetype` variants — `Ascent` (edge_activity≥0.6 short-circuit, 1503), else hue-quadrant → `Arch`/`NeighborTurn`/`Descent`/`Arch` (1506–1510). `InvertedArch`, `LeapStep`, `Pendulum`, `RisingSequence` are **defined with full contours** (chord_engine.rs:2325/2333/2335/2337; contour bodies at 2351/2357/2358/2360) but **never selected**. `resolve_motif` (chord_engine.rs:2379) already handles all 8 (it `match`es on `archetype.contour()`, and `RisingSequence` even has a 6-element contour the sampler at 2399–2417 already truncates/holds).

**(a) Change surface.** Two independent sub-levers:
- **Selection-spreading (composition.rs ONLY):** broaden `pick_archetype` (1500–1512) to reach all 8 by adding image conditions — e.g. `texture`/`complexity`/`colorfulness`/`palette_bimodality` (all already on `u`, all in scope) gate `InvertedArch`/`LeapStep`/`Pendulum`/`RisingSequence`. Pure Rust, ~10–20 LOC. The current `edge_activity≥0.6 → Ascent` short-circuit collapses every busy image to one contour — replacing it with a finer ladder is the highest-yield single change.
- **Vocabulary expansion (chord_engine.rs):** the 8 contours already exist; **no new contours are required** to triple the audible variety. Adding a *9th+* contour would be a `chord_engine.rs` edit (resolve_motif/contour set) — defer; not needed for the finding.

The selection ladder *could* be data-driven via a `SelectTable` (the pattern already used for `form`/`character`/`texture`/`prominence` at 1267–1391) — but `MotifArchetype` is a closed Rust enum resolved by `pick_archetype`, not a catalogue id, so a clean data-driven version needs a `motif_archetype` SelectTable + a `parse_archetype(&str)` mapper. That is a larger, more "right" refactor; the minimal slice is to broaden the existing Rust ladder.

**(b) Freeze-safety: SAFE for the composition.rs selection-spread.** `pick_archetype` runs at PLAN-BUILD time (1335), inside the `themes` block, gated by `theme_behaviour != "absent" && needs_theme`. `resolve_motif` is explicitly "build time only, never tick time" (chord_engine.rs:2367–2368). Changing *which* archetype is picked changes `ThemeSeed.motif` data, consumed downstream by the realizer's `theme_melody_pitch` — but that consumption is via the FROZEN realize path reading plan *data*, so engine.rs is untouched. **Caveat (flag loudly):** the realize path reads the motif, so a contour with a degree the realizer maps differently could in principle perturb output for theme-bearing sections — but identity/home sections that the `engine_equivalence` goldens pin carry `theme: None` (or the fixed hand-built plan), so the goldens are insensitive. If a future design wanted to add a NEW contour whose degree range exceeds the realizer's pitch-map assumptions, validate `theme_melody_pitch` first — still no engine.rs edit, but a chord_engine behavioral check.

**(c) Data-flow — already plumbed.** Every feature a richer ladder would read (`edge_activity`, `texture`, `complexity`, `colorfulness`, `dominant_hue`, `palette_bimodality`) is already on `ImageUnderstanding` and in scope at the `pick_archetype(u)` call (1335). Zero threading.

**(d) Test-net impact.** `engine_equivalence`: unaffected (fixed plan, no theme path). `runtime_reachability_s37`: unaffected (asserts spine, "never asserts per-step harmony realization" — its own header). `pattern_library_s34` / `figuration_s20` / `prominence_s23`: should be checked but operate on orchestration/bass/figuration catalogues, not motif archetype selection — likely orthogonal. `composition_s15`/`keyplan_*`: structural/key, motif-agnostic. No golden reads the *chosen archetype*, so spreading selection is behavior-additive on the theme path only.

**(e) Risk + size.** Risk LOW–MEDIUM (the one watch-item: a newly-reachable contour interacting with the realizer's degree→pitch map on theme-bearing sections; mitigated because all 8 contours already exist and `resolve_motif` already handles them). Size: **S (composition.rs ladder broadening, ~15 LOC)** for the minimal version; **M** if done as the cleaner data-driven `motif_archetype` SelectTable + parser.

---

## Finding 3 — Density too sparse

**Locus confirmed.** Two tiers:
- **Plan-time density constants (composition.rs:1225–1229):** `HOME_ENERGY_NEUTRAL=0.5`, `DENSITY_NEUTRAL=0.5`, `DENSITY_ENERGY_SPAN=0.30`, `DENSITY_FLOOR=0.35`, `DENSITY_CEIL=0.65`. These set per-section `Section.density` (composition.rs:1450 `f(e)=clamp(NEUTRAL + SPAN*(e-0.5), FLOOR, CEIL)`).
- **Realize-time density mechanics (chord_engine.rs):** `DENSITY_ACTIVITY_GAIN=0.5` (1403), `ARTIC_WINDOW_LO/HI=0.55/1.10` (1411–1412), `FILL_REST_ACTIVITY=0.10` (1436), `realize_rhythm` (1453) where `density_nudge=(ctx.section.density-0.5)*GAIN` (1485) nudges `edge_activity`, plus `walking_bass` (2019) and `figured_bed` (2206) which the rest/fill gates (1703/1860) drive off `edge_activity < FILL_REST_ACTIVITY`.

**(a) Change surface.** Sparseness has two distinct dials:
- **Constant/data tuning (freeze-safe knobs):** widen `DENSITY_ENERGY_SPAN` and/or raise `DENSITY_FLOOR`, and/or lower `FILL_REST_ACTIVITY` (currently 0.10 — images with `edge_activity<0.10` rest weak interior beats, which is a primary "too sparse" cause on calm images). These are **constant edits** in composition.rs / chord_engine.rs. NOT currently in mappings.json — they are `const` in Rust. Promoting them to `mappings.json` rows would be a small loader addition (composition has `composition` block, chord_engine reads `MappingTable`); the design should decide const-tune vs data-promote.
- **Structural (larger):** the `density_nudge` only biases `edge_activity` for *articulation/fill*; it does NOT add onsets/subdivisions. Genuinely *denser* output (more notes per beat) would need `realize_rhythm`/`figured_bed` to subdivide — a structural chord_engine change.

**(b) Freeze-safety: SPLIT — the byte-freeze hinge is `Section.density == 0.5` on identity sections.** The proof (chord_engine.rs:1400–1403, composition.rs:1448) is: `f(0.5)==DENSITY_NEUTRAL==0.5` exactly, and `(0.5-0.5)*GAIN==0.0`, so identity/home/home_only sections produce a zero nudge → realize path byte-stable → `engine_equivalence` goldens cannot move.
- **SAFE knobs:** anything that preserves `f(HOME_ENERGY_NEUTRAL)==0.5` and `density_nudge(0.5)==0.0`. `DENSITY_ENERGY_SPAN` (multiplies `(e-0.5)`, zero at e=0.5 ✓), `DENSITY_FLOOR`/`CEIL` **provided they still bracket 0.5** (FLOOR≤0.5≤CEIL — currently 0.35/0.65 ✓; raising FLOOR above 0.5 would clamp the identity 0.5 and **break the freeze** — flag loudly), `DENSITY_ACTIVITY_GAIN` (multiplies a delta that is 0 at identity ✓).
- **DANGER knobs:** `HOME_ENERGY_NEUTRAL`/`DENSITY_NEUTRAL` (move the neutral and identity sections stop being 0.5 → freeze breaks), and `FILL_REST_ACTIVITY` — this one is read against the **raw `edge_activity`**, NOT gated by the `density==0.5` identity hinge, so changing it can move output for **any** section including identity ones whose `edge_activity` straddles the threshold. **Whether the `engine_equivalence` fixtures' `edge_density` sit on either side of a moved `FILL_REST_ACTIVITY` must be checked before touching it** (the goldens use `edge_density:0.5` per chord_engine.rs:5577 → well above 0.10, so a *downward* move of FILL_REST is likely safe; an upward move toward 0.5 is not — VERIFY against the actual fixtures in the build dispatch).
- **None of these touch engine.rs.** All are composition.rs/chord_engine.rs constants on the (non-frozen-file but behaviorally-pinned) realize path. If a design wants to raise `DENSITY_FLOOR` above 0.5 or move `DENSITY_NEUTRAL`, that breaks the behavioral freeze and needs operator sign-off BEFORE the break.

**(c) Data-flow.** The density carrier is `Section.density`, set from `energy_i` (composition.rs:1417, the per-section `(offset, source_region_energy)` from `resolve_key_scheme`). On `home_only`/identity sections `energy_i==HOME_ENERGY_NEUTRAL==0.5` → density 0.5. So density is **only ever non-neutral on modulating/multi-region key schemes** — meaning on the common single-home image the density is *structurally pinned to 0.5 and the energy→density map never fires*. This is the deeper architectural cause of "too sparse on most images": the energy→density lever is **gated behind the key-scheme region machinery** and is dead for home-only images. Driving `Section.density` (or `realize_rhythm`'s baseline) from a whole-image feature (`edge_activity`/`texture`, both on `u`, both in scope at the section loop 1395) for home sections too would un-gate it — but that **moves the identity baseline off 0.5 and is the freeze-break path.** Flag loudly: making calm/home images denser *without* operator sign-off is only possible via the SAFE knobs (span/gain/floor-while-bracketing-0.5), which by construction leave the e=0.5 home case unchanged — i.e. the safe knobs **cannot** fix a home-only image's sparseness because that image *is* the frozen e=0.5 case. **This is the central tension of Finding 3.**

**(d) Test-net impact.** `engine_equivalence` 9/9: the binding constraint — any density change must keep the e=0.5 identity nudge at 0.0. `runtime_reachability_s37`/`composition_s15`: density-agnostic (spine/structure). `figuration_s20`/`pattern_library_s34`: exercise figured_bed/walking_bass paths — a `realize_rhythm` structural change risks these; constant-only tuning likely does not, but they must be re-run.

**(e) Risk + size.** SAFE-knob tuning: risk LOW, size **S (≈5 LOC const edits + re-run net)** — BUT delivers little on the common home-only image (the freeze hinge neutralizes it there, per (c)). Un-gating home-image density / structural subdivision: risk **HIGH** (freeze-break, operator sign-off, possible `engine_equivalence` golden re-baseline), size **M–L**. Finding 3 is the **least contained** — its satisfying fix collides with the byte-freeze.

---

## (f) Recommendation — Slice-1

**Finding 1 (image-derived home root) is the highest-leverage AND most-contained buildable Slice-1.**

Rationale:
- **Most contained:** single file (`composition.rs`), one line replaced (1302) + optional one JSON block, the driving feature (`dominant_hue`) already plumbed and already read one line up, no engine.rs touch, no main.rs touch, and a *proven-zero* test-net cost (no test asserts planner `home_root_midi`; `engine_equivalence` is hand-built and immune; `runtime_reachability` only strengthens). Smallest blast radius of the three.
- **Highest perceptual leverage per LOC:** "every image starts on the same pitch" is the most immediately audible identity failure of the three — distinct images currently differ in tempo/mode/length/articulation but all sound anchored to the same tonal center. Giving each image its own key center is a first-order identity gain.
- **Contrast with the others:** Finding 2 is comparably safe but lower-yield (motif variety is subtler than tonal center) and has a realizer-interaction watch-item; Finding 3's *satisfying* fix (denser home images) is exactly the freeze-break path and is explicitly NOT a clean Slice-1 — its safe subset is nearly inert on the common image.

**Rough shape of Slice-1 (Finding 1):**
1. Add `global.hue_to_root` (range map: hue 0..360 → root MIDI, e.g. a circle-of-fifths or hue-wheel→pitch-class mapping seated near C4) to `assets/mappings.json`, sibling of `hue_to_mode`.
2. In `composition.rs` `plan()`, replace `let home_root_midi = 60;` (1302) with `let home_root_midi = lookup_range_map(&mappings.global.hue_to_root, u.dominant_hue).and_then(|s| s.parse().ok()).unwrap_or(60);` (or a typed root map) — **keep `60` as the fallback** so a mappings.json without the new block reproduces today's behavior byte-for-byte (the same defensive default discipline `home_mode` uses at 1296).
3. Re-word the now-stale "home_root_midi seed = 60" docstrings in keyplan_k2a.rs:289/304 and composition.rs:1302 comment (consistency sweep — no assertion changes).
4. Run the full net; expect 9/9 `engine_equivalence` green unchanged, `runtime_reachability` green, `keyplan_*`/`composition_s15` green unchanged.

**Loud freeze flag for the synthesizer:** none of the three findings *requires* an engine.rs edit. If any later design wants root selection, motif degree-mapping, or density baseline moved into the tick kernel — or wants to raise `DENSITY_FLOOR` above 0.5 / move `DENSITY_NEUTRAL` (Finding 3's freeze hinge) — that breaks the byte-freeze (engine.rs sha or the `engine_equivalence` goldens) and **must get operator sign-off before the break.**

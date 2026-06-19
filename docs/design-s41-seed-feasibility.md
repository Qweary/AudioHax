# S41 — Deterministic Composition Seed: Feasibility Investigation (DESIGN-ONLY)

**Author:** Rust Architect (Specialist 1)
**Scope:** Design-only. No implementation code in this doc.
**Question:** Can a per-run deterministic seed (e.g. image-hash-derived) be threaded so the
composition path is REPRODUCIBLE, **without editing frozen `src/engine.rs`**?

## VERDICT: FREEZE-SAFE — buildable with ZERO edits to `engine.rs`.

The entire composition/plan path contains **exactly one** RNG draw, and it lives in a
**freeze-safe** file (`chord_engine.rs`), not in frozen `engine.rs`. A seed can be threaded to
it via a thread-local seed register that `pick_progression` reads, requiring no signature change
to any caller and no edit to `engine.rs`.

---

## 1. RNG-Draw Inventory (composition / plan path)

Exhaustive sweep of `thread_rng` / `rand::random` / `.choose` / `.shuffle` / `gen_range` over
`src/` (call sites, comments excluded):

| File:Line | Site | Frozen? | On composition path? |
|---|---|---|---|
| `src/chord_engine.rs:132` | `let mut rng = thread_rng();` | **NO — freeze-safe** | **YES — the only one** |
| `src/chord_engine.rs:133` | `choices.choose(&mut rng)` (consumes the rng above) | NO — freeze-safe | YES (same draw) |
| `src/bin/channel_sim.rs:77` | `StdRng::from_entropy()` | NO | No — acoustic/modem path, not composition |
| `src/modem.rs:1394` | `ChaCha8Rng::seed_from_u64(params.seed)` | NO | No — already-seeded acoustic channel (S7) |
| `src/modem.rs:2318+` | `seeded_payload` test scaffolding | NO | No — tests |

**There is exactly ONE non-deterministic draw on the entire composition/plan path:
`chord_engine.rs:132–133`.** Voice-leading / figure / species selection downstream is already
RNG-free and PICKS DETERMINISTICALLY (confirmed by the comments at `chord_engine.rs:3792` and the
PT-9 invariant at `chord_engine.rs:7893`).

### `engine.rs:378` is a comment, not a draw

The S9 boundary note often cited as "`engine.rs:378`" is a **doc-comment** describing
`pick_progression`'s behavior. It is NOT an RNG construction. `engine.rs` imports no `rand`
symbols (`grep "^use.*rand" src/engine.rs` → none). The two composition callers —
`engine.rs:409` (`set_features_global`) and `composition.rs:1558` (multi-section planner) — both
merely call `chord_engine.pick_progression(&mode)`; **neither constructs an RNG.**

---

## 2. The Seam

`pick_progression` is a method on `ChordEngine` (in freeze-safe `chord_engine.rs`). It constructs
`thread_rng()` **internally** (line 132) — the RNG is NOT passed in by the caller. So:

- The RNG construction site is **freeze-safe** (`chord_engine.rs`), not frozen.
- All callers (`engine.rs:409`, `composition.rs:1558`) route through the same method and pass NO
  rng — so changing how `pick_progression` *obtains* its randomness changes nothing for callers
  and touches no caller signature.

This is the best possible seam: the one draw is encapsulated behind a method, in a freeze-safe
file, and the frozen file only *calls* that method. We never thread a seed *into* frozen code.

**Why not just add an rng parameter to `pick_progression`?** That would change its signature, and
`engine.rs:409` calls it — forcing an edit to the frozen file. **Rejected.** The thread-local
register below threads the seed without any signature change.

---

## 3. The Build Shape (freeze-safe)

**Seed source register (new, in a freeze-safe module — e.g. `chord_engine.rs` or a tiny new
`src/seed.rs`):** a thread-local `Cell<Option<u64>>` holding the active composition seed, with a
freeze-safe setter (`set_composition_seed(Option<u64>)`) and reader.

**`pick_progression` change (freeze-safe, `chord_engine.rs:131–137`):** branch on the register —
- register holds `Some(seed)` → derive a per-call deterministic RNG with `ChaCha8Rng` (already a
  dependency, used by S7) keyed by `seed` mixed with a stable call counter / call-site salt so
  the *sequence* of progression picks within one composition is reproducible but not all-identical;
- register is `None` → fall through to today's `thread_rng()` path **unchanged** (behavior preserved
  byte-for-byte when no seed is set).

**Seed entry (CLI, freeze-safe `cli.rs`):** add `--seed <u64>` as `Option<u64>` (mirrors the
existing `--acoustic-seed u64` clap-derive precedent at `cli.rs:421`). Resolution at the
freeze-safe call boundary (`main.rs` / the render/play entry, NOT engine.rs):
- explicit `--seed N` → use `N`;
- absent → derive a stable seed from the **image hash** (the bytes are already loaded at the entry
  point), giving "same image ⇒ same composition" by default while different images still diverge;
- a `--no-seed` / `--random` escape hatch (optional) → set register to `None` = today's behavior.

The register is set once at the freeze-safe entry, before `engine.set_features_global(...)` /
the composition planner runs. `engine.rs` is never touched: it calls `pick_progression`, which
now reads the register.

**Determinism boundary note:** `thread_rng` was the *only* non-determinism on the plan path, so
seeding it makes the WHOLE plan reproducible. Because the per-section loop in
`composition.rs:1558` calls `pick_progression` multiple times, the register-keyed RNG must advance
deterministically across those calls (call counter in the register) so multi-section pieces are
reproducible too, not just collapsed to one repeated progression.

---

## 4. Freeze Verdict

- `src/engine.rs` — **UNTOUCHED.** sha256 `e50c7db1…2348261` holds.
- `engine_equivalence` 9/9 — **UNAFFECTED.** That net pins `decide_instrument_action` against a
  FIXED `&[StepPlan]` and never calls `set_features_global` / `pick_progression` (per the S9 §5
  note and the comments at `engine.rs:378` and `chord_engine.rs:7893`). The seed register is read
  only inside `pick_progression`, which the equivalence net does not exercise.
- All edits land in freeze-safe files: `chord_engine.rs` (the draw + register read), `cli.rs`
  (`--seed`), `main.rs`/entry (resolution + register set), optional new `src/seed.rs`.

---

## 5. Property-Test Spec

1. **PT-SEED-1 (reproducible):** same `--seed S` + same image ⇒ **byte-identical CompositionPlan**
   (assert on the serialized plan / the full `Vec<StepPlan>` + chord/root/tempo fields). Run the
   plan derivation twice in-process; assert equal.
2. **PT-SEED-2 (seed-sensitive):** same image, `--seed A` vs `--seed B` (A≠B) ⇒ plans differ
   (at least one progression/chord choice differs). Guards against the register being ignored.
3. **PT-SEED-3 (image-default determinism):** no `--seed`, same image twice ⇒ identical plan
   (image-hash default is stable); two DIFFERENT images ⇒ plans differ.
4. **PT-SEED-4 (absent-seed = legacy):** with the register `None` (the `--no-seed`/`--random`
   path), `pick_progression` takes the `thread_rng` branch — assert it still returns a valid
   progression from the family (shape test, NOT equality, exactly as today's tests do). This pins
   that today's behavior is preserved when unseeded.
5. **PT-SEED-5 (freeze guard, existing):** `engine_equivalence` 9/9 unchanged + the `engine.rs`
   sha256 guard.

Tests live in a freeze-safe test file (e.g. `tests/seed_s41.rs`); none touch engine.rs.

---

## 6. Value

Freeze-safe and high-leverage: seeding the single composition-path draw makes **every future A/B
listening test reproducible**. The ear-gate still compares structural character (not md5), but the
operator can now re-summon the *exact* composition for a given image+seed — so an A/B comparison
re-runs identically instead of drawing a fresh random progression each play, which is the root
friction this investigation set out to remove.

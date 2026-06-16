# Quality-Gate Review — BUG-01 (stuck MIDI note on abrupt exit)

**Scope reviewed:** uncommitted working tree against git HEAD `0fb0929`.
**Bug:** quitting AudioHax mid-note on the `--output midi` path leaves the note
sustaining forever in the external synth (a separate process that never receives a
note-off). The in-process `--output synth` path is self-healing (the cpal stream dies
with the process).

**VERDICT: PASS-WITH-ISSUES**

The fix is correct and complete for the primary reported scenario (Ctrl-C / SIGINT) and
all load-bearing claims were independently re-derived and confirmed. One genuine residual
stuck-note path remains (SIGTERM / SIGHUP — i.e. `kill <pid>` and terminal-window-close),
which the bug report's own wording ("window-close / kill") names. It is a one-line,
non-blocking improvement (enable the crate's `termination` feature). No collateral damage;
the music byte-freeze is intact; full suite green.

---

## 1. Panic bytes are correct — CONFIRMED (hand-derived)

`all_sound_off_messages()` body:

```rust
for ch in 0u8..16 {
    let status = 0xB0 | (ch & 0x0F);
    msgs.push([status, 123, 0]); // All Notes Off
    msgs.push([status, 120, 0]); // All Sound Off
}
```

Hand-check:

- `0u8..16` is exclusive → ch ∈ {0,1,…,15} = exactly **16** channels.
- For ch ≤ 15, `ch & 0x0F == ch`, so `status = 0xB0 | ch`, spanning 0xB0..=0xBF.
  High nibble = `0xB` = **Control Change** status. ✓
- 2 messages pushed per channel → 16 × 2 = **32 messages** total. ✓
- `123` (0x7B) = **All Notes Off**; `120` (0x78) = **All Sound Off** — both valid 7-bit
  CC numbers. Data byte `0` is the defined value for both. ✓ (CC 123 releases held notes
  into their release tail; CC 120 cuts anything still in its tail — sending both is belt
  and suspenders, correct for an external synth.)
- For **every** channel 0..=15, both `[0xB0|ch, 123, 0]` and `[0xB0|ch, 120, 0]` are
  present. ✓

Enumerated (ch: messages):

```
 0: [B0,7B,00] [B0,78,00]    8: [B8,7B,00] [B8,78,00]
 1: [B1,7B,00] [B1,78,00]    9: [B9,7B,00] [B9,78,00]
 2: [B2,7B,00] [B2,78,00]   10: [BA,7B,00] [BA,78,00]
 3: [B3,7B,00] [B3,78,00]   11: [BB,7B,00] [BB,78,00]
 4: [B4,7B,00] [B4,78,00]   12: [BC,7B,00] [BC,78,00]
 5: [B5,7B,00] [B5,78,00]   13: [BD,7B,00] [BD,78,00]
 6: [B6,7B,00] [B6,78,00]   14: [BE,7B,00] [BE,78,00]
 7: [B7,7B,00] [B7,78,00]   15: [BF,7B,00] [BF,78,00]
```

`MidiOut::all_sound_off` iterates these and calls `self.conn.send(&msg)` for each (msg is
`[u8;3]`, coerces to `&[u8]` — same call shape as the existing `note_on`/`note_off`). It
is **not dead** — it is called from `Drop`. The 5 inline `#[cfg(test)]` tests pin all of
the above and pass (count=32, every status nibble = 0xB0, only CC 123/120, both present
per channel, every data byte = 0, all 16 channel nibbles covered).

## 2. Drop is wired and correct — CONFIRMED

`impl Drop for MidiOut` calls `self.all_sound_off()` and, because `Drop` cannot return a
`Result`, logs-and-swallows any send error (`eprintln!`) rather than unwrapping/panicking
in drop. Correct — a panic in drop during unwind would abort.

`sink` is a `Box<dyn AudioSink>` and on the midi path holds a concrete `MidiOut`
(`impl AudioSink for MidiOut` in main.rs delegates straight to the inherent methods; no
wrapper type, no shadowing Drop). When the box drops, `Drop for MidiOut` runs through the
vtable. Drop fires on **normal return** (end of `main`'s `Ok(())`) and on any **early `?`
return** after the sink exists — both drop the owned box. Drop does NOT run on
`std::process::abort` or SIGKILL; that is exactly why the SIGINT handler exists.

## 3. SIGINT → graceful-return wiring — SOUND (for SIGINT)

- The handler closure captures only a cloned `Arc<AtomicBool>` and does
  `shutdown.store(true, SeqCst)`. It never touches the non-`Send` sink — correct and the
  only safe thing a signal-handler thread may do here.
- The handler is installed **unconditionally**, after sink selection, regardless of
  `--output synth|midi` / `--midi-virtual`. On the synth path the flag simply lets the
  loop break and the process exit cleanly (cpal self-heals).
- Loop break-points: the flag is polled (a) once per step at the top of the
  `for step_idx` loop, and (b) once per scheduled event inside the inner `for sev in
  events` loop, *before* the per-event `sleep`. The largest blocking interval between two
  flag checks is therefore a single event's sleep (≤ one note `hold_ms`), so a Ctrl-C
  lands within a bounded window — never "wait out the whole song." After the break the
  function falls through to `Ok(())` and returns, dropping the boxed sink → Drop → flush.
- `set_handler` returning `Err` is surfaced (`eprintln!`) and the program continues
  without the handler — acceptable degradation.

## 4. Fix resolves the reported bug — CONFIRMED for the `--output midi` path

Traced end to end: external synth holds a note → user hits Ctrl-C → handler sets the
AtomicBool → loop observes it and breaks (≤ one event) → `main` returns `Ok(())` → the
`Box<dyn AudioSink>` (a `MidiOut`) drops → `Drop` calls `all_sound_off` → CC 123 + CC 120
sent on all 16 channels over the live `MidiOutputConnection` → external synth releases the
note. The chain is complete and type-correct.

**Inspection-only (acceptable gap):** the actual byte-on-the-wire send to a *real* MIDI
port and the *live OS signal* delivery cannot be exercised in headless CI (no port, no
TTY). The byte content is fully unit-tested; the send path reuses the same
`conn.send(&[u8])` already proven by note_on/note_off in normal playback; the
signal→flag→break→drop control flow is verified by reading. This residual is reasonable.

## 5. Scope + no collateral damage — CONFIRMED

`git diff --name-only` is EXACTLY:

```
Cargo.lock
Cargo.toml
src/main.rs
src/midi_output.rs
```

- `git diff HEAD -- src/image_analysis.rs src/image_source.rs` → **EMPTY**. The earlier
  stray `cargo fmt` reflow of those two files was reverted; they now match HEAD. ✓
- No music/composition/sink/cli/tui/modem file is touched: `git diff HEAD --name-only`
  over engine.rs, chord_engine.rs, composition.rs, mapping_loader.rs, mappings.json,
  synth_sink.rs, cli.rs, tui.rs, modem.rs → **EMPTY**. ✓
- The only non-functional churn in main.rs is two rustfmt-driven reflows of a pre-existing
  match arm and a long line, both inert.

## 6. Music byte-freeze intact + full suite green — CONFIRMED (real counts)

- `cargo build` (default) → exit **0** (only pre-existing dead-code/unused-var warnings in
  unrelated bins/lib; none from the changed files).
- `cargo test --test engine_equivalence` → **9 passed; 0 failed**, including
  `test_full_golden_sweep_is_byte_identical` — goldens unmoved.
- `cargo test --bin audiohax` → **5 passed; 0 failed** (the new MIDI-panic tests).
- `cargo test --lib` → **151 passed; 0 failed**.
- Integration spot-checks: `figuration_s20` → **8 passed; 0 failed**;
  `saliency_s18` → **12 passed; 0 failed**.

## 7. Hygiene — CONFIRMED

- Word-boundary grep for swarm/framework codenames over the two changed source files →
  **no matches**.
- `ctrlc` is a maintained, cross-platform crate (the locked 3.5.2 pulls `nix` on Unix,
  `windows-sys` on Windows, `dispatch2` on macOS) — matches the project's OS-agnostic
  posture. `ctrlc = "3.4"` is a reasonable floor.
- rustfmt: `src/midi_output.rs` is clean (rustfmt `--check` exit 0). `src/main.rs` is
  clean when checked in isolation (rustfmt `--check` via stdin → exit 0, no diff). NOTE:
  running `rustfmt --check src/main.rs` as a path returns non-zero, but ONLY because
  rustfmt follows `mod` declarations into the *pre-existing*, *unchanged* unformatted
  `image_analysis.rs` / `image_source.rs`; main.rs's own content is clean. This is a
  property of the repo's prior formatting state, not of this change.

---

## Residual stuck-note gap (the one issue behind PASS-WITH-ISSUES)

The handler is `ctrlc::set_handler` with **no `termination` feature** enabled
(`Cargo.toml`: `ctrlc = "3.4"`, no `features`). By default the crate intercepts **only
SIGINT (Ctrl-C)** on Unix. Therefore:

- **Ctrl-C (SIGINT):** handled → graceful return → Drop → all-sound-off flush. ✓
- **`kill <pid>` (SIGTERM):** NOT handled → process terminates without running Drop →
  **note stays stuck** in the external synth.
- **terminal-window-close (SIGHUP):** NOT handled → same outcome → **note stays stuck**.

The bug report's wording explicitly lists "Ctrl-C / window-close / kill", so SIGTERM and
SIGHUP are in-scope of the reported defect and remain unaddressed on the `--output midi`
path. This is the only path by which a non-`-9` quit can still leak a sustaining note.

**Suggested (one-line, non-blocking) remedy:** enable the crate's termination feature so
the same flag→break→Drop path also catches SIGTERM/SIGHUP —

```toml
ctrlc = { version = "3.4", features = ["termination"] }
```

(SIGKILL / `kill -9` / `process::abort` are inherently uncatchable and are correctly out
of scope.)

## Non-blocking nits

- Comment in main.rs says Drop "fires the all-sound-off **panic**" — "panic" is MIDI-slang
  for the all-notes/sound-off blast, not a Rust panic; fine in context but a reader could
  misread it.
- `all_sound_off` is `pub` but only used internally (Drop + tests); harmless.

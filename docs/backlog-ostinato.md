# Backlog — Ostinato / Pedal-Point capability gap

**Filed:** S52 (Bundle 1 honesty cleanup). **Status:** OPEN — candidate future build, not scheduled.

## What

The engine has **no ostinato or pedal-point capability**. A repeating fixed
figure under changing harmony (ostinato) and a sustained/repeated anchor pitch
(pedal point) are real expressive axes the image→music pipeline currently cannot
produce.

## Provenance

This gap surfaced during the S51 dormancy audit (`docs/audit-s51-dormancy.md`,
finding **D-FINEDET**). The removed legacy `fine_detail` schema block contained a
`shape_to_ostinato` mapping naming three never-implemented targets —
`PedalPoint`, `AscendingOstinato`, `DescendingOstinato`. The audit confirmed
**zero** of those symbols exist anywhere in the engine: the axis was schema'd in
the S1 era but never built, then the schema was deleted in S52 as dead
loaded-but-unread data. Removing the dead schema does not remove the capability —
the capability was never there. This note preserves the idea so the deletion does
not erase the intent.

## Why it is a real axis (not rot)

Unlike the rest of `fine_detail` (pitch/velocity/jaggedness, all superseded by
the live motif/contour + chord-realizer path), ostinato/pedal has **no live
substitute** in the engine. It is a genuine latent gap, not redundant legacy.

## Reuse pointer (if/when built)

A future build would most naturally land alongside the macro-rhythm work
(fix-direction-2, Bundle 3): an image-derived repeating cell is conceptually a
sibling of `pick_rhythm_cell`'s "rhythmic motto," and a pedal anchor is a key/bass
behavior that could reuse the `Knob` + `SelectTable` plumbing
(`composition.rs:817-839`). No new infra is required to *select* an ostinato/pedal
axis; the missing piece is a consumer that lays the figure down in the realizer.

## Disposition

Logged for the operator's consideration. Not in scope for S52 (cleanup) or the
S53 Bundle-3 kickoff (un-gate `pick_rhythm_cell` + build the meter consumer)
unless the operator elects to fold it in.

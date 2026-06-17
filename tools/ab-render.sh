#!/bin/sh
# ab-render.sh — render ONE image through several audio configs to out_*.wav files
# for instant, apples-to-apples A/B comparison (S31). Offline: no audio device needed.
#
#   tools/ab-render.sh <IMAGE> [SOUNDFONT.sf2]
#
# Each config renders the SAME composition deterministically (the engine plan is fixed
# once per render invocation; the only knob that changes between files is the synth
# config), so the WAVs differ ONLY by soundfont/reverb/gain. Open them side-by-side in
# any audio editor and listen.
#
# WHAT YOU GET (in the current directory):
#   out_baseline.wav     bundled GM font, reverb on,  gain 1.0   (today's default sound)
#   out_dry.wav          bundled GM font, reverb OFF, gain 1.0   (how much is reverb?)
#   out_gain.wav         bundled GM font, reverb on,  gain 1.5   (does headroom help?)
#   out_font.wav         YOUR font,       reverb on,  gain 1.0   (the big lever; if given)
#   out_font_gain.wav    YOUR font,       reverb on,  gain 1.5   (likely "good default")
#
# DROP-IN FONT: pass any uncompressed .sf2 as the 2nd arg, e.g. FluidR3_GM.sf2 or the
# uncompressed MuseScore_General.sf2 (rustysynth is SF2-only — NOT SF3/compressed).
# Fetch one yourself (both are MIT-licensed) and point this script at it.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if [ "$#" -lt 1 ]; then
  echo "usage: tools/ab-render.sh <IMAGE> [SOUNDFONT.sf2]" >&2
  echo "  renders <IMAGE> through several audio configs to out_*.wav for A/B." >&2
  exit 2
fi
IMAGE="$1"
FONT="${2:-}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "ab-render: 'cargo' not on PATH. Run: export PATH=\"\$HOME/.cargo/bin:\$PATH\"" >&2
  exit 127
fi
if [ ! -f "$IMAGE" ]; then
  echo "ab-render: image not found: $IMAGE" >&2
  exit 2
fi

cd "$SCRIPT_DIR"

run() {
  # run <out.wav> <extra render flags...>
  out="$1"; shift
  echo "ab-render: -> $out"
  cargo run --release -q -- render "$IMAGE" --wav "$out" "$@"
}

# Always-available bundled-font configs (A0 / A1 / A2 from the research matrix).
run out_baseline.wav
run out_dry.wav      --reverb off
run out_gain.wav     --gain 1.5

# Font A/B only if the owner supplied an .sf2 (B0 / B1).
if [ -n "$FONT" ]; then
  if [ ! -f "$FONT" ]; then
    echo "ab-render: soundfont not found: $FONT (skipping font rows)" >&2
  else
    run out_font.wav      --soundfont "$FONT"
    run out_font_gain.wav --soundfont "$FONT" --gain 1.5
  fi
else
  echo "ab-render: no soundfont given — rendered bundled-font configs only."
  echo "ab-render: pass an uncompressed .sf2 (e.g. FluidR3_GM.sf2) as arg 2 for the font A/B."
fi

echo "ab-render: done. Compare the out_*.wav files in your audio editor."

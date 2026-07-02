#!/bin/sh
# ab-motif-s62.sh — render EVERY example image through the CURRENT engine at a fixed
# seed + default synth config, so each output differs ONLY by the image. This is the
# S62 "before" set for the cross-piece MOTIVIC-DIVERSITY investigation: play them
# back-to-back and listen for whether each image yields its OWN tune or the same
# motif with minor variations.
#
#   tools/ab-motif-s62.sh <OUTDIR> [before|after]
#
# Deterministic: --seed 42 fixes the composition, so re-runs are byte-identical and an
# "after" set (post-fix) is an apples-to-apples A/B against this "before" set.
set -eu

LABEL="${2:-before}"
OUT="${1:-motif-ab-s62}"
SEED=42

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "ab-motif: 'cargo' not on PATH. Run: export PATH=\"\$HOME/.cargo/bin:\$PATH\"" >&2
  exit 127
fi

mkdir -p "$OUT"

# one release build up front so per-image renders are fast + share artifacts
echo "ab-motif: building (release, default pure-Rust features) ..."
cargo build --release --bin audiohax >/dev/null

for img in assets/images/AudioHaxImg1.jpg assets/images/AudioHaxImg2.jpg \
           assets/images/AudioHaxImg3.jpg assets/images/example.jpg \
           assets/images/Lena.png assets/images/magicstudio-art.jpg; do
  base=$(basename "$img" | sed 's/\.[^.]*$//')
  wav="$OUT/${LABEL}_${base}.wav"
  echo "ab-motif: -> $wav"
  cargo run --release -q --bin audiohax -- render "$img" --wav "$wav" --seed "$SEED"
done

echo "ab-motif: DONE. $LABEL WAVs in $OUT/ — play them back-to-back and listen for per-image motivic identity."
ls -la "$OUT"/

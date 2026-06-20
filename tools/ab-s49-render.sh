#!/bin/bash
# ab-s49-render.sh — build-version A/B render for S49 slice 2 (per-role rhythmic identity).
# AFTER = current working tree (S49); BEFORE = HEAD (S48, 92930de).
# Renders the 6 probe images at --seed 42, bundled soundfont both sides.
set -eu
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
cd "$(dirname "$0")/.."
OUT="$(cd .. && pwd)/ab-s49-wavs"
mkdir -p "$OUT"

IMG_AudioHaxImg1=assets/images/AudioHaxImg1.jpg
IMG_AudioHaxImg2=assets/images/AudioHaxImg2.jpg
IMG_AudioHaxImg3=assets/images/AudioHaxImg3.jpg
IMG_example=assets/images/example.jpg
IMG_Lena=assets/images/Lena.png
IMG_magicstudio=assets/images/magicstudio-art.jpg
NAMES="AudioHaxImg1 AudioHaxImg2 AudioHaxImg3 example Lena magicstudio"
outname() { case "$1" in magicstudio) echo magicstudio-art;; *) echo "$1";; esac; }

render_all() {
  local prefix="$1"
  cargo build --release -q
  for n in $NAMES; do
    local var="IMG_$n"; local path="${!var}"; local out="$OUT/${prefix}_$(outname "$n").wav"
    cargo run --release -q -- render "$path" --wav "$out" --seed 42 2>&1 | grep -E "wrote|error:" || { echo "RENDER FAILED: $n"; exit 1; }
  done
}

echo "### AFTER (S49 current tree)"
render_all AFTER

echo "### stash S49 -> S48 (HEAD)"
git stash push -q -m "s49-ab-render" -- src/chord_engine.rs src/composition.rs tests/texture_s17.rs tests/variety_scorecard_s45.rs
trap 'git stash pop -q 2>/dev/null || true' EXIT
git log --oneline -1
echo "### BEFORE (S48)"
render_all BEFORE

echo "### restore S49"
git stash pop -q
trap - EXIT
cargo build --release -q
echo "### engine.rs freeze:"; sha256sum src/engine.rs
echo "### result:"; ls -la "$OUT"

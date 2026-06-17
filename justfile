# AudioHax convenience recipes. `just` is optional — the `./play` script is the
# canonical frictionless entrypoint; these recipes just wrap it. Run `just --list`.
#
# Every recipe ensures cargo's userspace bin dir is on PATH (cargo is installed
# under $HOME/.cargo on this box).

# Play the bundled sample image through the in-process synth (zero routing).
play:
    PATH="$HOME/.cargo/bin:$PATH" ./play

# Play a specific image (and any extra `audiohax play` flags), e.g.
#   just play-image assets/images/example.jpg
#   just play-image my.png --instruments 6
play-image image *flags:
    PATH="$HOME/.cargo/bin:$PATH" ./play {{image}} {{flags}}

# Show the wrapper's help.
help:
    ./play --help

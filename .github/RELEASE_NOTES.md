First public release: a Rust rewrite of the original Python `famitracker2midi`.

## Highlights

- Dependency-free Rust binary (no Python or `mido`), builds and runs offline.
- Automatic expansion-chip support: VRC6, VRC7, FDS, MMC5, N163 (1-8 waves)
  and 5B, detected from the `EXPANSION` header.
- Real CLI: `famitracker2midi <input.txt> [output.mid]`, `-o/--output`,
  `--help`, `--version`.
- Multi-song exports produce one MIDI file per song.
- Noise, DPCM and MMC5 PCM routed to General MIDI percussion.

## Install

Download the binary for your platform below, or build from source:

    cargo build --release

## Credits

Original Python implementation by [Psudonem](https://github.com/Psudonem) (MIT).
Rust port by Murapixel.

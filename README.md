# famitracker2midi

[![ci](https://github.com/murapixel/famitracker2midi-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/murapixel/famitracker2midi-rs/actions/workflows/ci.yml)

Rust CLI that converts FamiTracker / Dn-FamiTracker text exports into MIDI
files. Expansion sound chips are detected and mapped automatically from the
`EXPANSION` header, so N163, VRC6, VRC7, FDS, MMC5 and 5B projects all convert
without extra flags.

## Why this port

This is a Rust rewrite of the original
[famitracker2midi](https://github.com/Psudonem/famitracker2midi), a Python 3
script written by **Psudonem**. The original required installing
[mido](https://mido.readthedocs.io/), editing filenames in the source before
each run, and only converted the five standard 2A03 channels.

The goals of this port:

- A single, dependency-free binary (no Python or `mido`), building and running
  fully offline.
- A real command-line interface instead of hard-coded filenames.
- Automatic support for FamiTracker / Dn-FamiTracker expansion chips
  (VRC6, VRC7, FDS, MMC5, N163 and 5B), detected from the `EXPANSION` header.
- Cleaner timing, note-cut, volume-column and multi-song handling.

The text-export interpretation and MIDI conventions intentionally follow the
original implementation.

## Supported chips

| Chip | Channels | Mapping |
| --- | --- | --- |
| 2A03 | 2 pulse, triangle, noise, DPCM | melodic programs + GM percussion |
| VRC6 | 2 pulse, sawtooth | melodic |
| VRC7 | 6 FM | melodic |
| FDS | 1 | melodic |
| MMC5 | 2 pulse, PCM | melodic + percussion |
| N163 | 1-8 waves (`N163CHANNELS`) | melodic |
| 5B (Sunsoft) | 3 square | melodic |

Noise, DPCM and MMC5 PCM are routed to MIDI channel 10 as General MIDI
percussion. Everything else gets a synthesized-lead style program.

## Build

```sh
cargo build --release
```

The crate has no external dependencies, so it builds offline.

## Usage

```sh
cargo run -- "cha cha.txt" out.mid
# or, after building:
./target/release/famitracker2midi "cha cha.txt" out.mid
```

- `INPUT` defaults to `musica.txt`; `OUTPUT` defaults to `INPUT` with a `.mid` extension.
- `-o/--output <FILE>` sets the output explicitly.
- Multi-song exports (repeated `TRACK`) write `out.1.mid`, `out.2.mid`, ...

Export your song from FamiTracker with **File -> Export text** and pass that
file in.

## Layout

- `src/parser.rs` — text-export parser (header, orders, patterns, multi-song).
- `src/chips.rs` — expansion bitfield -> channel layout and MIDI programs.
- `src/convert.rs` — order/pattern traversal and note events.
- `src/midi.rs` — dependency-free Standard MIDI File writer.
- `src/main.rs` — CLI.
- `tests/` — integration tests, including a VRC6 + N163 fixture.

## Tests

```sh
cargo test
cargo fmt
cargo clippy --all-targets -- -D warnings
```

## Credits

- Original Python implementation: [Psudonem](https://github.com/Psudonem)
  ([famitracker2midi](https://github.com/Psudonem/famitracker2midi)).
- Rust port: Murapixel.
- MIT License; see [LICENSE](LICENSE) (original copyright 2020 Psudonem).

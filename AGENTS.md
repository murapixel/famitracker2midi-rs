# AGENTS.md

Rust CLI that converts FamiTracker / Dn-FamiTracker `.txt` exports to MIDI.
Expansion chips (VRC6, VRC7, FDS, MMC5, N163, 5B) are detected and mapped
automatically from the `EXPANSION` header.

## Build / test / lint
- `cargo build`, `cargo test`, `cargo run -- <input.txt> <output.mid>`
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` (both currently clean).
- The crate has **zero external dependencies** (only `std`, including a hand-written
  SMF writer in `src/midi.rs`). Keep it that way unless there is a strong reason; it
  builds offline and there is no `Cargo.lock` committed or vendored registry.
- Do not add a Python/`mido` dependency; the Rust binary is the only implementation.

## Run
- `cargo run -- "cha cha.txt" out.mid` or `./target/debug/famitracker2midi "cha cha.txt" out.mid`
- Default input `musica.txt`; default output is the input path with `.mid`.
- Multi-song exports (`TRACK` repeated) produce one file per song: `out.1.mid`, `out.2.mid`, ...
- Sample: `cha cha.txt` -> committed reference `cha cha.mid`. Do NOT overwrite the reference; write to a scratch path. The reference was made by the old 96-ticks/row Python script, so byte/note counts differ; that is expected.
- Verify a result with `cargo test` or by parsing the output as an SMF; there is no external MIDI library in-repo.

## Source layout
- `src/parser.rs` — line parser + `NoteToken`/`note_to_midi` logic. `Song`/`Pattern` types, multi-song handling, header fields (`MACHINE`, `EXPANSION`, `N163CHANNELS`).
- `src/chips.rs` — `ChannelKind`, `channel_layout()` (expansion bitfield -> ordered channel list), `expansion_names()`.
- `src/convert.rs` — `convert_song()`: order/pattern traversal, note events, MIDI channel assignment.
- `src/midi.rs` — dependency-free SMF type-1 writer (`MidiFile`, `TrackBuilder`, `write_vlq`).
- `src/main.rs` — CLI arg parsing and human-readable report.
- `tests/sample.rs` + `tests/data/n163.txt` — integration tests incl. multichip N163.

## FamiTracker text quirks (parser relies on these)
- Header order: `MACHINE 0|1`, `EXPANSION <bitfield>`, `N163CHANNELS 1..8`, then one or more `TRACK`.
- `TRACK <length> <speed> <tempo> "<name>"` — order is length, speed, tempo.
- `COLUMNS : n n ...` — one entry **per channel**; the value is the channel's effect-column count (number of trailing effect tokens).
- `ORDER`/`PATTERN` ids and volume tokens are hex; note tokens look like `C-4` / `D#3` / `E-3`; noise pitches look like `0-#` .. `F-#` and are treated as percussion.
- `ROW` channel fields are separated by `" : "`; per-channel token order is `note instrument volume effect...`.
- `...` = no event, `---`/`===` = note cut (both round-trip to the same `NoteToken::Cut`).
- `EXPANSION` is a bitfield: 1=VRC6, 2=VRC7, 4=FDS, 8=MMC5, 16=N163, 32=S5B. Channel order always follows this bit order after the 5 2A03 channels.

## Conversion conventions (non-obvious)
- Timing is fixed: `TICKS_PER_ROW = 120`, `TICKS_PER_BEAT = 480`; `row_seconds = speed * 2.5 / tempo`.
- Melodic channels are assigned MIDI channels 0..15 skipping 9, wrapping if there are more than 15.
- Noise, DPCM and MMC5 PCM channels are routed to MIDI channel 10 (index 9) as GM percussion; `channels_used` still reports their source indices.
- Programs come from `ChannelKind::program()` in `src/chips.rs`; percussion channels emit no program change.
- Note names map as `12 + 12*octave + semitone` (so `C-1` = 24); N163 uses the same mapping.

//! Turn a parsed [`Song`] into a MIDI file.

use crate::chips::{self, ChannelKind};
use crate::midi::{MidiFile, TrackBuilder, TICKS_PER_BEAT, TICKS_PER_ROW};
use crate::parser::{parse_note_token, volume_to_velocity, Module, NoteToken, Song};

/// Everything the CLI needs to report about one converted song.
#[derive(Debug, Clone)]
pub struct Conversion {
    pub midi: MidiFile,
    /// Zero-based source channel indices that produced at least one note.
    pub channels_used: Vec<usize>,
    /// Chip layout derived from the `EXPANSION` header.
    pub layout: Vec<ChannelKind>,
    pub patterns: usize,
    pub orders: usize,
    pub rows_per_pattern: usize,
    pub row_seconds: f64,
    pub duration: f64,
}

/// Convert one song using the module's global expansion configuration.
pub fn convert_song(module: &Module, song: &Song) -> Conversion {
    let layout = chips::channel_layout(module.expansion, module.n163_channels);
    let channel_count = song.columns.len();
    let rows_per_pattern = song.length.max(1);

    let speed = song.speed.max(1) as f64;
    let tempo = song.tempo.max(1) as f64;
    let row_seconds = speed * 2.5 / tempo;
    let microseconds_per_beat =
        (row_seconds * TICKS_PER_BEAT as f64 / TICKS_PER_ROW as f64 * 1_000_000.0) as u64;

    let channels_used: Vec<usize> = (0..channel_count)
        .filter(|&channel| channel_has_note(song, channel))
        .collect();

    let mut midi = MidiFile::new(1, TICKS_PER_BEAT);
    let mut tempo_written = false;
    let mut next_melodic_channel: u8 = 0;

    for &channel in &channels_used {
        let kind = match layout.get(channel).copied() {
            Some(kind) => kind,
            None => continue, // more columns than the chip layout knows about
        };
        let percussion = kind.is_percussion();
        let midi_channel = if percussion {
            9
        } else {
            allocate_melodic_channel(&mut next_melodic_channel)
        };

        let mut track = TrackBuilder::new();
        track.track_name(0, &kind.label());
        if !tempo_written {
            track.tempo(0, microseconds_per_beat);
            track.time_signature(0);
            tempo_written = true;
        }
        if let Some(program) = kind.program() {
            track.program_change(0, midi_channel, program);
        }

        let mut current_note: Option<u8> = None;
        let mut tick: u64 = 0;

        for order in &song.orders {
            let pattern = order.get(channel).and_then(|pid| song.patterns.get(pid));
            for row_index in 0..rows_per_pattern {
                let tokens = pattern
                    .and_then(|rows| rows.get(row_index))
                    .and_then(|row| row.get(channel));
                let token = tokens
                    .and_then(|fields| fields.first())
                    .map(String::as_str)
                    .unwrap_or("...");

                let velocity = tokens
                    .and_then(|fields| fields.get(2))
                    .map(|volume| volume_to_velocity(volume))
                    .unwrap_or(100);

                match parse_note_token(token) {
                    NoteToken::Cut => {
                        if let Some(note) = current_note.take() {
                            track.note_off(tick, midi_channel, note);
                        }
                    }
                    NoteToken::Note(note) => {
                        if let Some(old) = current_note.take() {
                            track.note_off(tick, midi_channel, old);
                        }
                        let note = if percussion {
                            to_percussion(note)
                        } else {
                            note
                        };
                        track.note_on(tick, midi_channel, note, velocity);
                        current_note = Some(note);
                    }
                    NoteToken::Noise(value) => {
                        if let Some(old) = current_note.take() {
                            track.note_off(tick, midi_channel, old);
                        }
                        let note = 36u8.saturating_add(value).min(81);
                        track.note_on(tick, midi_channel, note, velocity);
                        current_note = Some(note);
                    }
                    NoteToken::None => {}
                }

                tick += TICKS_PER_ROW;
            }
        }

        if let Some(note) = current_note.take() {
            track.note_off(tick, midi_channel, note);
        }
        track.end(tick);
        midi.tracks.push(track.build());
    }

    if midi.tracks.is_empty() {
        let mut track = TrackBuilder::new();
        track.tempo(0, microseconds_per_beat);
        track.time_signature(0);
        track.end(0);
        midi.tracks.push(track.build());
    }

    Conversion {
        midi,
        channels_used,
        layout,
        patterns: song.patterns.len(),
        orders: song.orders.len(),
        rows_per_pattern: song.length,
        row_seconds,
        duration: song.orders.len() as f64 * song.length as f64 * row_seconds,
    }
}

fn channel_has_note(song: &Song, channel: usize) -> bool {
    for order in &song.orders {
        let pattern = match order.get(channel).and_then(|pid| song.patterns.get(pid)) {
            Some(rows) => rows,
            None => continue,
        };
        for row in pattern {
            if let Some(token) = row.get(channel).and_then(|fields| fields.first()) {
                if !matches!(parse_note_token(token), NoteToken::None) {
                    return true;
                }
            }
        }
    }
    false
}

fn allocate_melodic_channel(next: &mut u8) -> u8 {
    loop {
        let candidate = *next % 16;
        *next = next.wrapping_add(1);
        if candidate != 9 {
            return candidate;
        }
    }
}

/// DPCM/sample channels are mapped onto GM percussion notes.
fn to_percussion(note: u8) -> u8 {
    note.clamp(35, 81)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_module;

    #[test]
    fn converts_plain_song() {
        let text = "\
EXPANSION 0
TRACK 4 6 150 \"t\"
COLUMNS : 1 1 1 1 1
ORDER 00 : 00 00 00 00 00
PATTERN 00
ROW 00 : C-4 00 F ... : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ...
ROW 01 : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ...
";
        let module = parse_module(text);
        let conversion = convert_song(&module, &module.songs[0]);
        assert_eq!(conversion.channels_used, vec![0]);
        assert_eq!(conversion.midi.tracks.len(), 1);
        assert!(!conversion.midi.tracks[0].is_empty());
    }

    #[test]
    fn converts_n163_channel() {
        let text = "\
MACHINE 0
EXPANSION 16
N163CHANNELS 4
TRACK 4 6 150 \"n163\"
COLUMNS : 1 1 1 1 1 1 1 1 1
ORDER 00 : 00 00 00 00 00 00 00 00 00
PATTERN 00
ROW 00 : C-4 00 F ... : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ... : E-5 00 F ... : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ...
";
        let module = parse_module(text);
        let conversion = convert_song(&module, &module.songs[0]);
        assert_eq!(conversion.layout.len(), 9);
        assert_eq!(conversion.channels_used, vec![0, 5]);
        assert_eq!(conversion.midi.tracks.len(), 2);
        assert_eq!(conversion.midi.to_bytes()[0..4], *b"MThd");
    }

    #[test]
    fn noise_goes_to_percussion_channel() {
        let text = "\
EXPANSION 0
TRACK 2 6 150 \"noise\"
COLUMNS : 1 1 1 1 1
ORDER 00 : 00 00 00 00 00
PATTERN 00
ROW 00 : ... .. . ... : ... .. . ... : ... .. . ... : 1-# 00 F ... : ... .. . ...
";
        let module = parse_module(text);
        let conversion = convert_song(&module, &module.songs[0]);
        assert_eq!(conversion.channels_used, vec![3]);
        let track = &conversion.midi.tracks[0];
        // 0x99 = note on, MIDI channel 9.
        assert!(track.windows(3).any(|w| w == [0x99, 37, 126]));
    }
}

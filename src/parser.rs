use std::collections::HashMap;

/// A single pattern: rows -> channels -> tokens.
pub type Pattern = Vec<Vec<Vec<String>>>;

/// One song/track inside an exported module.
#[derive(Debug, Default, Clone)]
pub struct Song {
    pub name: String,
    /// Rows per pattern (the `<length>` field of `TRACK`).
    pub length: usize,
    pub speed: u32,
    pub tempo: u32,
    /// Effect-column count per channel (from `COLUMNS`).
    pub columns: Vec<usize>,
    /// `orders[order_position][channel]` = pattern id.
    pub orders: Vec<Vec<usize>>,
    /// pattern id -> rows.
    pub patterns: HashMap<usize, Pattern>,
}

/// A parsed text export, including global header fields.
#[derive(Debug, Default, Clone)]
pub struct Module {
    /// 0 = NTSC, 1 = PAL.
    pub machine: u8,
    /// Bitfield: 1=VRC6, 2=VRC7, 4=FDS, 8=MMC5, 16=N163, 32=S5B.
    pub expansion: u8,
    /// Channels used by N163 (1..=8). 0 means "not specified" -> default 8.
    pub n163_channels: u8,
    pub songs: Vec<Song>,
}

/// Result of decoding a note column token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteToken {
    /// Standard pitched note, already converted to a MIDI note number.
    Note(u8),
    /// 2A03 noise pitch (`0-#` .. `F-#`), raw 0..15 value.
    Noise(u8),
    /// `---` or `===`.
    Cut,
    /// `...` / empty / unrecognised.
    None,
}

/// Parse a full FamiTracker text export into a [`Module`].
pub fn parse_module(text: &str) -> Module {
    let mut module = Module::default();
    let mut songs: Vec<Song> = Vec::new();
    let mut current: Option<Song> = None;
    let mut current_pattern: Option<usize> = None;

    for raw in text.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (head, rest) = split_command(trimmed);

        match head {
            "TRACK" => {
                if let Some(song) = current.take() {
                    songs.push(song);
                }
                current_pattern = None;
                let mut song = Song {
                    speed: 6,
                    tempo: 150,
                    length: 64,
                    ..Song::default()
                };
                let mut parts = rest.split_whitespace();
                if let Some(v) = parts.next().and_then(|t| t.parse::<usize>().ok()) {
                    song.length = v;
                }
                if let Some(v) = parts.next().and_then(|t| t.parse::<u32>().ok()) {
                    song.speed = v;
                }
                if let Some(v) = parts.next().and_then(|t| t.parse::<u32>().ok()) {
                    song.tempo = v;
                }
                song.name = extract_quoted(rest);
                current = Some(song);
            }
            "MACHINE" => module.machine = first_int(rest).unwrap_or(0) as u8,
            "EXPANSION" => module.expansion = first_int(rest).unwrap_or(0) as u8,
            "N163CHANNELS" => module.n163_channels = first_int(rest).unwrap_or(8) as u8,
            "COLUMNS" => {
                if let Some(song) = current.as_mut() {
                    let payload = after_colon(rest);
                    song.columns = payload
                        .split_whitespace()
                        .filter_map(|t| t.parse::<usize>().ok())
                        .collect();
                }
            }
            "ORDER" => {
                if let Some(song) = current.as_mut() {
                    let payload = after_colon(rest);
                    let ids: Vec<usize> = payload
                        .split_whitespace()
                        .filter_map(|t| u32::from_str_radix(t, 16).ok())
                        .map(|v| v as usize)
                        .collect();
                    if !ids.is_empty() {
                        song.orders.push(ids);
                    }
                }
            }
            "PATTERN" => {
                if let Some(song) = current.as_mut() {
                    if let Some(id) = rest.split_whitespace().next().and_then(parse_hex) {
                        let id = id as usize;
                        current_pattern = Some(id);
                        song.patterns.entry(id).or_default();
                    }
                }
            }
            "ROW" => {
                if let (Some(song), Some(pid)) = (current.as_mut(), current_pattern) {
                    if let Some((row_part, groups_part)) = rest.split_once(':') {
                        let row_index = parse_hex(row_part).unwrap_or(0) as usize;
                        let groups: Vec<Vec<String>> = groups_part
                            .split(" : ")
                            .map(|group| group.split_whitespace().map(String::from).collect())
                            .collect();
                        let rows = song.patterns.entry(pid).or_default();
                        if rows.len() <= row_index {
                            rows.resize_with(row_index + 1, Vec::new);
                        }
                        rows[row_index] = groups;
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(song) = current.take() {
        songs.push(song);
    }
    module.songs = songs;
    module
}

/// Decode a note-column token.
///
/// Recognised shapes: `C-4`, `D#3`, `A-0` .. `B-9` and the noise forms
/// `0-#` .. `F-#`. `---` / `===` mean note cut.
pub fn parse_note_token(token: &str) -> NoteToken {
    if token == "---" || token == "===" {
        return NoteToken::Cut;
    }
    if token.is_empty() || token == "..." {
        return NoteToken::None;
    }
    let bytes = token.as_bytes();
    if bytes.len() < 3 {
        return NoteToken::None;
    }

    // Noise pitches: <hex digit> - #
    if bytes[1] == b'-' && bytes[2] == b'#' {
        if let Some(value) = (bytes[0] as char).to_digit(16) {
            return NoteToken::Noise(value as u8);
        }
    }

    let base = match bytes[0] {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        b'B' => 11,
        _ => return NoteToken::None,
    };
    if bytes[1] != b'-' && bytes[1] != b'#' {
        return NoteToken::None;
    }
    let octave = match bytes[2] {
        b'0'..=b'9' => (bytes[2] - b'0') as i32,
        _ => return NoteToken::None,
    };
    let semitone = base + if bytes[1] == b'#' { 1 } else { 0 };
    let midi = 12 + 12 * octave + semitone;
    if (0..=127).contains(&midi) {
        NoteToken::Note(midi as u8)
    } else {
        NoteToken::None
    }
}

/// FamiTracker volume column (one hex digit) to MIDI velocity.
pub fn volume_to_velocity(token: &str) -> u8 {
    if token.len() == 1 {
        if let Some(digit) = token.chars().next().and_then(|c| c.to_digit(16)) {
            return ((digit as f64 / 15.0 * 126.0).round() as u8).max(1);
        }
    }
    100
}

fn parse_hex(token: &str) -> Option<u32> {
    u32::from_str_radix(token.trim(), 16).ok()
}

fn after_colon(rest: &str) -> &str {
    rest.split_once(':').map(|(_, right)| right).unwrap_or(rest)
}

fn first_int(rest: &str) -> Option<u32> {
    rest.split_whitespace()
        .find_map(|t| t.trim_matches(':').parse::<u32>().ok())
}

fn extract_quoted(text: &str) -> String {
    if let (Some(start), Some(end)) = (text.find('"'), text.rfind('"')) {
        if end > start {
            return text[start + 1..end].to_string();
        }
    }
    String::new()
}

fn split_command(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(idx) => (&line[..idx], line[idx..].trim_start()),
        None => (line, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# FamiTracker text export 0.4.2
MACHINE 0
EXPANSION 0
TRACK  64   7 150 \"New song\"
COLUMNS : 1 1 1 1 1

ORDER 00 : 00 00 00 00 00

PATTERN 00
ROW 00 : E-3 00 . ... : D-1 00 5 ... : D-4 00 . ... : ... .. . ... : ... .. . ...
ROW 01 : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ... : ... .. . ...
";

    #[test]
    fn parses_header_and_song() {
        let module = parse_module(SAMPLE);
        assert_eq!(module.expansion, 0);
        assert_eq!(module.machine, 0);
        assert_eq!(module.songs.len(), 1);
        let song = &module.songs[0];
        assert_eq!(song.name, "New song");
        assert_eq!(song.length, 64);
        assert_eq!(song.speed, 7);
        assert_eq!(song.tempo, 150);
        assert_eq!(song.columns, vec![1, 1, 1, 1, 1]);
        assert_eq!(song.orders, vec![vec![0, 0, 0, 0, 0]]);
        assert_eq!(song.patterns[&0][0].len(), 5);
    }

    #[test]
    fn parses_n163_channels() {
        let text =
            "EXPANSION 16\nN163CHANNELS 4\nTRACK 64 6 150 \"x\"\nCOLUMNS : 1 1 1 1 1 1 1 1 1\n";
        let module = parse_module(text);
        assert_eq!(module.expansion, 16);
        assert_eq!(module.n163_channels, 4);
    }

    #[test]
    fn note_tokens() {
        assert_eq!(parse_note_token("C-1"), NoteToken::Note(24));
        assert_eq!(parse_note_token("D#3"), NoteToken::Note(51));
        assert_eq!(parse_note_token("A-4"), NoteToken::Note(69));
        assert_eq!(parse_note_token("1-#"), NoteToken::Noise(1));
        assert_eq!(parse_note_token("F-#"), NoteToken::Noise(15));
        assert_eq!(parse_note_token("---"), NoteToken::Cut);
        assert_eq!(parse_note_token("==="), NoteToken::Cut);
        assert_eq!(parse_note_token("..."), NoteToken::None);
    }

    #[test]
    fn volume_scale() {
        assert_eq!(volume_to_velocity("F"), 126);
        assert_eq!(volume_to_velocity("0"), 1);
        assert_eq!(volume_to_velocity("."), 100);
    }
}

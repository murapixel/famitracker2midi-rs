//! Minimal dependency-free Standard MIDI File (format 1) writer.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Ticks per quarter note used for all generated files.
pub const TICKS_PER_BEAT: u16 = 480;
/// Internal resolution: one FamiTracker row = 120 ticks (1/4 beat).
pub const TICKS_PER_ROW: u64 = 120;

/// A complete MIDI file before serialisation.
#[derive(Debug, Clone)]
pub struct MidiFile {
    pub format: u16,
    pub division: u16,
    /// Raw `MTrk` chunk bytes (including the `MTrk` magic and length).
    pub tracks: Vec<Vec<u8>>,
}

impl MidiFile {
    pub fn new(format: u16, division: u16) -> Self {
        MidiFile {
            format,
            division,
            tracks: Vec::new(),
        }
    }

    /// Serialise the whole file to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.tracks.iter().map(Vec::len).sum::<usize>());
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&self.format.to_be_bytes());
        out.extend_from_slice(&(self.tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.division.to_be_bytes());
        for track in &self.tracks {
            out.extend_from_slice(track);
        }
        out
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.to_bytes())
    }
}

impl Default for MidiFile {
    fn default() -> Self {
        MidiFile::new(1, TICKS_PER_BEAT)
    }
}

/// Accumulates events for one track and serialises them with delta times.
#[derive(Debug, Default, Clone)]
pub struct TrackBuilder {
    /// (tick, priority, raw event bytes). Lower priority sorts first at equal ticks.
    events: Vec<(u64, u8, Vec<u8>)>,
}

impl TrackBuilder {
    pub fn new() -> Self {
        TrackBuilder::default()
    }

    pub fn note_on(&mut self, tick: u64, channel: u8, note: u8, velocity: u8) {
        self.events
            .push((tick, 1, vec![0x90 | (channel & 0x0F), note, velocity]));
    }

    pub fn note_off(&mut self, tick: u64, channel: u8, note: u8) {
        self.events
            .push((tick, 0, vec![0x80 | (channel & 0x0F), note, 0]));
    }

    pub fn program_change(&mut self, tick: u64, channel: u8, program: u8) {
        self.events
            .push((tick, 0, vec![0xC0 | (channel & 0x0F), program & 0x7F]));
    }

    pub fn track_name(&mut self, tick: u64, name: &str) {
        let bytes = name.as_bytes();
        let mut data = vec![0xFF, 0x03];
        write_vlq(&mut data, bytes.len() as u32);
        data.extend_from_slice(bytes);
        self.events.push((tick, 0, data));
    }

    pub fn tempo(&mut self, tick: u64, microseconds_per_beat: u64) {
        let value = microseconds_per_beat.min(0xFF_FFFF) as u32;
        let bytes = value.to_be_bytes();
        self.events.push((
            tick,
            0,
            vec![0xFF, 0x51, 0x03, bytes[1], bytes[2], bytes[3]],
        ));
    }

    pub fn time_signature(&mut self, tick: u64) {
        // 4/4 with 24 clocks per quarter and 8 thirty-seconds per quarter.
        self.events
            .push((tick, 0, vec![0xFF, 0x58, 0x04, 4, 2, 24, 8]));
    }

    pub fn end(&mut self, tick: u64) {
        self.events.push((tick, 0, vec![0xFF, 0x2F, 0x00]));
    }

    /// Serialise into a complete `MTrk` chunk.
    pub fn build(mut self) -> Vec<u8> {
        // Stable sort keeps insertion order for equal (tick, priority).
        self.events
            .sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut data = Vec::new();
        let mut last_tick = 0u64;
        for (tick, _priority, bytes) in &self.events {
            write_vlq(&mut data, (tick - last_tick) as u32);
            data.extend_from_slice(bytes);
            last_tick = *tick;
        }

        let mut chunk = Vec::with_capacity(data.len() + 8);
        chunk.extend_from_slice(b"MTrk");
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(&data);
        chunk
    }
}

/// Variable-length quantity encoding used for delta times and lengths.
pub fn write_vlq(out: &mut Vec<u8>, mut value: u32) {
    let mut buffer = [0u8; 4];
    let mut index = 0;
    buffer[index] = (value & 0x7F) as u8;
    index += 1;
    value >>= 7;
    while value > 0 {
        buffer[index] = ((value & 0x7F) as u8) | 0x80;
        index += 1;
        value >>= 7;
    }
    for i in (0..index).rev() {
        out.push(buffer[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlq_roundtrip_shapes() {
        let mut out = Vec::new();
        write_vlq(&mut out, 0);
        assert_eq!(out, vec![0x00]);

        out.clear();
        write_vlq(&mut out, 0x7F);
        assert_eq!(out, vec![0x7F]);

        out.clear();
        write_vlq(&mut out, 0x80);
        assert_eq!(out, vec![0x81, 0x00]);

        out.clear();
        write_vlq(&mut out, 0x0FFF_FFFF);
        assert_eq!(out, vec![0xFF, 0xFF, 0xFF, 0x7F]);
    }

    #[test]
    fn header_is_well_formed() {
        let mut file = MidiFile::new(1, TICKS_PER_BEAT);
        let mut track = TrackBuilder::new();
        track.end(0);
        file.tracks.push(track.build());
        let bytes = file.to_bytes();
        assert_eq!(&bytes[0..4], b"MThd");
        assert_eq!(&bytes[14..18], b"MTrk");
    }
}

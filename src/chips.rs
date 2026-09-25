//! Expansion-chip aware channel layout.
//!
//! The `EXPANSION` header is a bitfield: 1=VRC6, 2=VRC7, 4=FDS, 8=MMC5,
//! 16=N163, 32=S5B. FamiTracker always lays channels out in that same order
//! after the five 2A03 channels, so the full channel map can be reconstructed
//! from the header alone.

/// Logical type of a single exported channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Square1,
    Square2,
    Triangle,
    Noise,
    Dpcm,
    Vrc6Square1,
    Vrc6Square2,
    Vrc6Saw,
    Vrc7Fm(u8),
    Fds,
    Mmc5Square1,
    Mmc5Square2,
    Mmc5Dpcm,
    N163(u8),
    S5bSquare1,
    S5bSquare2,
    S5bSquare3,
}

impl ChannelKind {
    /// Human readable channel label, used for MIDI track names.
    pub fn label(&self) -> String {
        match self {
            ChannelKind::Square1 => "Square 1".into(),
            ChannelKind::Square2 => "Square 2".into(),
            ChannelKind::Triangle => "Triangle".into(),
            ChannelKind::Noise => "Noise".into(),
            ChannelKind::Dpcm => "DPCM".into(),
            ChannelKind::Vrc6Square1 => "VRC6 Square 1".into(),
            ChannelKind::Vrc6Square2 => "VRC6 Square 2".into(),
            ChannelKind::Vrc6Saw => "VRC6 Sawtooth".into(),
            ChannelKind::Vrc7Fm(n) => format!("VRC7 FM {}", n + 1),
            ChannelKind::Fds => "FDS".into(),
            ChannelKind::Mmc5Square1 => "MMC5 Square 1".into(),
            ChannelKind::Mmc5Square2 => "MMC5 Square 2".into(),
            ChannelKind::Mmc5Dpcm => "MMC5 PCM".into(),
            ChannelKind::N163(n) => format!("N163 Wave {}", n + 1),
            ChannelKind::S5bSquare1 => "5B Square 1".into(),
            ChannelKind::S5bSquare2 => "5B Square 2".into(),
            ChannelKind::S5bSquare3 => "5B Square 3".into(),
        }
    }

    /// Channels routed to the GM percussion channel (MIDI channel 10).
    pub fn is_percussion(&self) -> bool {
        matches!(
            self,
            ChannelKind::Noise | ChannelKind::Dpcm | ChannelKind::Mmc5Dpcm
        )
    }

    /// General MIDI program for melodic channels; `None` for percussion.
    pub fn program(&self) -> Option<u8> {
        let program = match self {
            ChannelKind::Square1 => 80,
            ChannelKind::Square2 => 81,
            ChannelKind::Triangle => 73,
            ChannelKind::Vrc6Square1 => 80,
            ChannelKind::Vrc6Square2 => 81,
            ChannelKind::Vrc6Saw => 81,
            ChannelKind::Vrc7Fm(_) => 81,
            ChannelKind::Fds => 81,
            ChannelKind::Mmc5Square1 => 80,
            ChannelKind::Mmc5Square2 => 81,
            ChannelKind::N163(_) => 80,
            ChannelKind::S5bSquare1 => 80,
            ChannelKind::S5bSquare2 => 81,
            ChannelKind::S5bSquare3 => 80,
            ChannelKind::Noise | ChannelKind::Dpcm | ChannelKind::Mmc5Dpcm => return None,
        };
        Some(program)
    }
}

/// Build the channel layout for an expansion bitfield + N163 channel count.
pub fn channel_layout(expansion: u8, n163_channels: u8) -> Vec<ChannelKind> {
    let mut layout = vec![
        ChannelKind::Square1,
        ChannelKind::Square2,
        ChannelKind::Triangle,
        ChannelKind::Noise,
        ChannelKind::Dpcm,
    ];
    if expansion & 0x01 != 0 {
        layout.push(ChannelKind::Vrc6Square1);
        layout.push(ChannelKind::Vrc6Square2);
        layout.push(ChannelKind::Vrc6Saw);
    }
    if expansion & 0x02 != 0 {
        for i in 0..6 {
            layout.push(ChannelKind::Vrc7Fm(i));
        }
    }
    if expansion & 0x04 != 0 {
        layout.push(ChannelKind::Fds);
    }
    if expansion & 0x08 != 0 {
        layout.push(ChannelKind::Mmc5Square1);
        layout.push(ChannelKind::Mmc5Square2);
        layout.push(ChannelKind::Mmc5Dpcm);
    }
    if expansion & 0x10 != 0 {
        let count = if n163_channels == 0 {
            8
        } else {
            n163_channels.min(8)
        };
        for i in 0..count {
            layout.push(ChannelKind::N163(i));
        }
    }
    if expansion & 0x20 != 0 {
        layout.push(ChannelKind::S5bSquare1);
        layout.push(ChannelKind::S5bSquare2);
        layout.push(ChannelKind::S5bSquare3);
    }
    layout
}

/// Comma-separated list of the chips enabled by an expansion bitfield.
pub fn expansion_names(expansion: u8) -> String {
    let mut names = Vec::new();
    if expansion & 0x01 != 0 {
        names.push("VRC6");
    }
    if expansion & 0x02 != 0 {
        names.push("VRC7");
    }
    if expansion & 0x04 != 0 {
        names.push("FDS");
    }
    if expansion & 0x08 != 0 {
        names.push("MMC5");
    }
    if expansion & 0x10 != 0 {
        names.push("N163");
    }
    if expansion & 0x20 != 0 {
        names.push("5B");
    }
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join("+")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_2a03() {
        let layout = channel_layout(0, 0);
        assert_eq!(layout.len(), 5);
        assert_eq!(layout[0], ChannelKind::Square1);
    }

    #[test]
    fn n163_only() {
        let layout = channel_layout(16, 4);
        assert_eq!(layout.len(), 9);
        assert_eq!(layout[5], ChannelKind::N163(0));
        assert_eq!(layout[8], ChannelKind::N163(3));
    }

    #[test]
    fn n163_defaults_to_eight() {
        assert_eq!(channel_layout(16, 0).len(), 5 + 8);
    }

    #[test]
    fn multichip_order() {
        // VRC6 | FDS | N163 = 1 + 4 + 16 = 21
        let layout = channel_layout(1 | 4 | 16, 2);
        let labels: Vec<String> = layout.iter().map(|k| k.label()).collect();
        assert_eq!(labels[5], "VRC6 Square 1");
        assert_eq!(labels[8], "FDS");
        assert_eq!(labels[9], "N163 Wave 1");
        assert_eq!(layout.len(), 5 + 3 + 1 + 2);
    }

    #[test]
    fn percussion_classification() {
        assert!(ChannelKind::Noise.is_percussion());
        assert!(ChannelKind::Dpcm.is_percussion());
        assert!(!ChannelKind::N163(0).is_percussion());
        assert_eq!(ChannelKind::N163(0).program(), Some(80));
    }
}

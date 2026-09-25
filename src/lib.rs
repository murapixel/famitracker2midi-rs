//! FamiTracker / Dn-FamiTracker text-export to MIDI converter.
//!
//! The parser understands the 0.4.x text format and the Dn-FamiTracker
//! extensions (`EXPANSION`, `N163CHANNELS`, multi-song files). Channel layout
//! is derived automatically from the expansion-chip bitfield so N163, VRC6,
//! VRC7, FDS, MMC5 and 5B channels are converted without extra flags.

pub mod chips;
pub mod convert;
pub mod midi;
pub mod parser;

pub use convert::{convert_song, Conversion};
pub use parser::{parse_module, Module, NoteToken, Song};

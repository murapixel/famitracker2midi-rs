use std::path::PathBuf;

use famitracker2midi::chips::{channel_layout, expansion_names};
use famitracker2midi::convert::convert_song;
use famitracker2midi::parse_module;

fn repo_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

#[test]
fn converts_committed_sample() {
    let text = std::fs::read_to_string(repo_path("cha cha.txt")).expect("read sample");
    let module = parse_module(&text);

    assert_eq!(module.expansion, 0);
    assert_eq!(module.songs.len(), 1);
    assert_eq!(expansion_names(module.expansion), "none");

    let song = &module.songs[0];
    assert_eq!(song.length, 64);
    assert_eq!(song.speed, 7);
    assert_eq!(song.tempo, 150);

    let conversion = convert_song(&module, song);
    assert_eq!(conversion.channels_used, vec![0, 1, 2]);
    assert_eq!(conversion.layout.len(), 5);
    assert_eq!(conversion.midi.tracks.len(), 3);
    assert_eq!(&conversion.midi.to_bytes()[0..4], b"MThd");
}

#[test]
fn reparses_multichip_layout() {
    let layout = channel_layout(1 | 2 | 4 | 8 | 16 | 32, 8);
    let labels: Vec<String> = layout.iter().map(|kind| kind.label()).collect();
    assert_eq!(labels.len(), 5 + 3 + 6 + 1 + 3 + 8 + 3);
    assert!(labels.contains(&"N163 Wave 8".to_string()));
    assert!(labels.contains(&"VRC7 FM 6".to_string()));
    assert!(labels.contains(&"5B Square 3".to_string()));
}

#[test]
fn multichip_export_converts_n163_notes() {
    let fixture = repo_path("tests/data/n163.txt");
    let text = std::fs::read_to_string(&fixture).expect("read fixture");
    let module = parse_module(&text);

    assert_eq!(module.expansion, 1 | 16); // VRC6 + N163
    assert_eq!(module.n163_channels, 4);

    let conversion = convert_song(&module, &module.songs[0]);
    // 5x 2A03 + 3x VRC6 + 4x N163.
    assert_eq!(conversion.layout.len(), 12);
    assert!(conversion.channels_used.contains(&5)); // VRC6 square 1
    assert!(conversion.channels_used.contains(&8)); // N163 wave 1
    assert!(conversion.midi.tracks.len() >= 2);
}

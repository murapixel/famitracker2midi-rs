use std::path::{Path, PathBuf};
use std::process::ExitCode;

use famitracker2midi::chips::expansion_names;
use famitracker2midi::convert::{convert_song, Conversion};
use famitracker2midi::parser::{parse_module, Module, Song};

const USAGE: &str = "\
famitracker2midi - convert FamiTracker/Dn-FamiTracker text exports to MIDI

USAGE:
    famitracker2midi [OPTIONS] [INPUT] [OUTPUT]

ARGS:
    INPUT     input .txt export (default: musica.txt)
    OUTPUT    output .mid file (default: INPUT with .mid extension)

OPTIONS:
    -o, --output <FILE>    explicit output file
    -h, --help             print this help
    -V, --version          print version

Multi-song exports are written as OUTPUT with a `.N` suffix per song.
Expansion chips (VRC6, VRC7, FDS, MMC5, N163, 5B) are detected automatically
from the EXPANSION header.
";

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("try `famitracker2midi --help`");
            return ExitCode::from(2);
        }
    };

    let text = match std::fs::read_to_string(&args.input) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", args.input.display());
            return ExitCode::FAILURE;
        }
    };

    let module = parse_module(&text);
    if module.songs.is_empty() {
        eprintln!("error: no TRACK found in {}", args.input.display());
        return ExitCode::FAILURE;
    }

    let multiple = module.songs.len() > 1;
    for (index, song) in module.songs.iter().enumerate() {
        let conversion = convert_song(&module, song);
        let output = if multiple {
            indexed_path(&args.output, index + 1)
        } else {
            args.output.clone()
        };
        if let Err(err) = conversion.midi.save(&output) {
            eprintln!("error: cannot write {}: {err}", output.display());
            return ExitCode::FAILURE;
        }
        print_report(
            &module,
            song,
            &conversion,
            &args.input,
            &output,
            index + 1,
            multiple,
        );
    }

    ExitCode::SUCCESS
}

struct Args {
    input: PathBuf,
    output: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut iter = std::env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("famitracker2midi {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-o" | "--output" => {
                output = Some(
                    iter.next()
                        .ok_or_else(|| "--output requires a value".to_string())?,
                );
            }
            other if other.starts_with('-') && other.len() > 1 => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else if output.is_none() {
                    output = Some(other.to_string());
                } else {
                    return Err(format!("unexpected argument: {other}"));
                }
            }
        }
    }

    let input = input.unwrap_or_else(|| "musica.txt".to_string());
    let output = output.unwrap_or_else(|| default_output(&input));
    Ok(Args {
        input: PathBuf::from(input),
        output: PathBuf::from(output),
    })
}

fn default_output(input: &str) -> String {
    Path::new(input)
        .with_extension("mid")
        .to_string_lossy()
        .into_owned()
}

fn indexed_path(output: &Path, index: usize) -> PathBuf {
    let stem = output
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    let extension = output
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "mid".to_string());
    output.with_file_name(format!("{stem}.{index}.{extension}"))
}

fn print_report(
    module: &Module,
    song: &Song,
    conversion: &Conversion,
    input: &Path,
    output: &Path,
    song_number: usize,
    multiple: bool,
) {
    if multiple {
        println!("song {song_number}/{}: {}", module.songs.len(), song.name);
    } else if !song.name.is_empty() {
        println!("song: {}", song.name);
    }
    println!(
        "chip: {} (EXPANSION 0x{:02X})",
        expansion_names(module.expansion),
        module.expansion
    );
    let layout: Vec<String> = conversion
        .layout
        .iter()
        .enumerate()
        .map(|(index, kind)| format!("{index}:{}", kind.label()))
        .collect();
    println!("channels: {}", layout.join(" "));
    println!("channels converted: {:?}", conversion.channels_used);
    println!(
        "patterns: {}  orders: {}  rows/pattern: {}",
        conversion.patterns, conversion.orders, conversion.rows_per_pattern
    );
    println!(
        "speed: {}  tempo: {}  row: {:.4}s  duration: {:.2}s",
        song.speed, song.tempo, conversion.row_seconds, conversion.duration
    );
    println!("wrote {} (from {})", output.display(), input.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_output_name() {
        assert_eq!(default_output("song.txt"), "song.mid");
        assert_eq!(default_output("/tmp/a b.txt"), "/tmp/a b.mid");
    }

    #[test]
    fn indexed_output() {
        assert_eq!(
            indexed_path(Path::new("/tmp/out.mid"), 2),
            PathBuf::from("/tmp/out.2.mid")
        );
    }
}

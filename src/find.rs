use crate::detect::{is_sound_file, is_transcript};
use std::env::current_dir;
use std::io::Result;
use std::path::PathBuf;

/// Examines input files and directories and returns the relevant ones,
/// in lexicographical order of filenames.
///
/// Only transcripts in F4 RTF format are considered relevant.
pub fn collect_transcripts(from: Vec<PathBuf>, recursive: bool) -> Result<Vec<PathBuf>> {
    find(from, recursive, |p| is_transcript(p))
}

/// Collect files that sound like interview filenames, e.g. mp3 files.
pub fn collect_interviews(from: Vec<PathBuf>, recursive: bool) -> Result<Vec<PathBuf>> {
    find(from, recursive, |p| Ok(is_sound_file(p)))
}

fn find<F: Fn(&PathBuf) -> Result<bool>>(
    from: Vec<PathBuf>,
    recursive: bool,
    mut predicate: F,
) -> Result<Vec<PathBuf>> {
    let mut found = vec![];
    if from.is_empty() {
        // default to current working directory if no paths specified
        add_where(&mut found, current_dir()?, recursive, predicate)?;
    } else {
        for input in from {
            predicate = add_where(&mut found, input, recursive, predicate)?;
        }
    }
    found.sort_unstable();
    Ok(found)
}

fn add_where<F: Fn(&PathBuf) -> Result<bool>>(
    into: &mut Vec<PathBuf>,
    input: PathBuf,
    recursive: bool,
    mut predicate: F,
) -> Result<F> {
    if input.is_dir() {
        for entry in input.read_dir()? {
            let entry = entry?.path();
            if entry.is_file() || (entry.is_dir() && recursive) {
                predicate = add_where(into, entry, recursive, predicate)?
            }
        }
    } else if input.is_file() && predicate(&input)? {
        into.push(input)
    }

    Ok(predicate)
}

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::args::Split;
use crate::find::collect_interviews;
use crate::paths::path_as_str;

use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

const SEGMENT_TIME: &str = "00:05:00";

pub fn split(opts: Split) -> Result<()> {
    let paths = collect_interviews(opts.input_files, opts.recursive)?;
    if paths.is_empty() {
        return Err(Error::NoInterviews);
    }

    let preferred_output_dir : Option<&Path> = opts.output_directory.as_ref().map(AsRef::as_ref);
    for path in paths {
        let output_dir = output_directory_or_interview_parent(preferred_output_dir, &path)?;
        split_interview(&path, output_dir)?;
    }
    Ok(())
}

fn output_directory_or_interview_parent<'a>(preferred_output_directory: Option<&'a Path>, interview_path: &'a Path) -> Result<Option<&'a Path>> {
    let output_dir = preferred_output_directory
        .or_else(|| interview_path.parent())
        // parent of a parent-less path seems to be "", but we want None then
        .filter(|path| !path.as_os_str().is_empty());

    if let Some(output_dir) = output_dir {
        // ensure exists and is a directory, unless we write into current directory
        // and do not have a parent directory to write into, which will always work
        if !output_dir.is_dir() {
            return Err(Error::output_directory_not_found(output_dir.into()));
        }
    }

    Ok(output_dir)
}

fn split_interview(interview: &Path, output_dir: Option<&Path>) -> Result<()> {
    let interview_str = path_as_str(&interview)?;
    let pattern = segment_pattern(output_dir, interview)?;
    let pattern = (&pattern).to_str().ok_or_else(|| Error::EncodingError)?;
    let args = [
        "-i",
        interview_str,
        "-c",
        "copy",
        "-map",
        "0",
        "-segment_time",
        SEGMENT_TIME,
        "-f",
        "segment",
        pattern,
    ];
    let status = Command::new("ffmpeg")
        .args(&args)
        .status()
        .map_err(Error::FfmpegIo)?;
    if !status.success() {
        return Err(Error::FfmpegStatus);
    }
    Ok(())
}

/// Output pattern for use with ffmpeg.
/// 
/// Will use the give output directory, if any, otherwise the pattern will
/// be for a relative path.
fn segment_pattern(output_directory: Option<&Path>, interview: &Path) -> Result<PathBuf> {
    let interview_stem = interview
        .file_stem()
        .unwrap() // unwrap is safe, collect_interviews does not return empty filenames
        .to_str()
        .ok_or_else(|| Error::EncodingError)?;

    let mut pattern = PathBuf::new();
    if let Some(output_directory) = output_directory {
        pattern.push(output_directory);
    }

    pattern.push(format!("{}-%03d", interview_stem));
    if let Some(extension) = interview.extension() {
        pattern.set_extension(extension);
    }

    Ok(pattern)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("no interview recordings found")]
    NoInterviews,
    #[error("output directory for record segments not found or not a directory: {0}")]
    OutputDirectoryNotFound(PathBuf),
    #[error("input filename was not valid UTF-8, other encodings are not supported")]
    EncodingError,
    #[error("failed to invoke ffmpeg to split the interview files, install with your favorite package manager or on Windows download from https://ffmpeg.org/download.html#build-windows and add to your \"Path\" environment variable")]
    FfmpegIo(std::io::Error),
    #[error("splitting interviews with ffmpeg failed")]
    FfmpegStatus,
}

impl Error {
    fn output_directory_not_found(output_directory: PathBuf) -> Self {
        Self::OutputDirectoryNotFound(output_directory)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn output_directory_for_interview_with_parent_dir() {
        let preferred_output_dir = None;
        let interview = Path::new("testdata/interview.mp3");
        assert_eq!(
            output_directory_or_interview_parent(preferred_output_dir, interview).unwrap().unwrap(),
            Path::new("testdata")
        );
    }

    #[test]
    fn output_directory_for_interview_without_parent_dir_but_existing_preferred_dir() {
        let preferred_output_dir = Some(Path::new("src"));
        let interview = Path::new("testdata/interview.mp3");
        assert!(preferred_output_dir.unwrap().is_dir(), "Expected for test that \"{:?}\" is an existing directory", preferred_output_dir);
        assert_eq!(
            output_directory_or_interview_parent(preferred_output_dir, interview).unwrap().unwrap(),
            Path::new("src")
        );
    }

    #[test]
    fn output_directory_for_interview_without_parent_dir_and_without_preferred_dir() {
        let preferred_output_dir = None;
        let interview = Path::new("interview.mp3");
        assert_eq!(
            output_directory_or_interview_parent(preferred_output_dir, interview).unwrap(),
            None
        );
    }

    #[test]
    fn pattern_for_interview_without_output_dir() {
        let output_dir = None;
        let interview = Path::new("interview.mp3");
        assert_eq!(
            segment_pattern(output_dir, interview).unwrap(),
            PathBuf::from("interview-%03d.mp3")
        );
    }

    #[test]
    fn pattern_for_interview_with_output_dir() {
        let output_dir = Some(Path::new("src"));
        let interview = Path::new("testdata/interview.mp3");
        assert_eq!(
            segment_pattern(output_dir, interview).unwrap(),
            PathBuf::from("src/interview-%03d.mp3")
        );
    }
}

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

    for path in paths {
        split_interview(&path)?;
    }
    Ok(())
}

fn split_interview(interview: &Path) -> Result<()> {
    let interview_str = path_as_str(&interview)?;
    let pattern = segment_pattern(interview)?;
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

fn segment_pattern(interview: &Path) -> Result<PathBuf> {
    let interview_dir = interview.parent();
    let interview_stem = interview
        .file_stem()
        .unwrap() // unwrap is safe, collect_interviews does not return empty filenames
        .to_str()
        .ok_or_else(|| Error::EncodingError)?;

    let mut pattern = PathBuf::new();
    if let Some(interview_dir) = interview_dir {
        pattern.push(interview_dir);
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
    #[error("input filename was not valid UTF-8, other encodings are not supported")]
    EncodingError,
    #[error("failed to invoke ffmpeg to split the interview files, install with your favorite package manager or on Windows download from https://ffmpeg.org/download.html#build-windows and add to your \"Path\" environment variable")]
    FfmpegIo(std::io::Error),
    #[error("splitting interviews with ffmpeg failed")]
    FfmpegStatus,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pattern_for_interview_with_parent_dir() {
        let interview = &Path::new("testdata/interview.mp3");
        assert_eq!(
            segment_pattern(interview).unwrap(),
            PathBuf::from("testdata/interview-%03d.mp3")
        )
    }
}

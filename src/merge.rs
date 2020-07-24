use std::fs::File;
use std::path::PathBuf;

use crate::args::Merge;
use crate::find::collect_transcripts;
use crate::transcript::{write_merged_transcript, Error as TranscriptError, Transcript};

use log::warn;
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

pub fn merge(opts: Merge) -> Result<()> {
    let mut transcripts = collect_transcripts(opts.input_segments, opts.recursive)?
        .into_iter()
        .filter_map(|path| match Transcript::from_file(&path) {
            Ok(transcript) => Some(transcript),
            Err(err) => {
                warn!(
                    "failed to load transcript {}, skipping, cause: {}",
                    path.display(),
                    err
                );
                None
            }
        })
        .peekable();

    // need at least on transcript
    if transcripts.peek().is_none() {
        return Err(Error::NoTranscripts);
    }

    // write merged transcript while lazily loading them
    match opts.output_file {
        Some(output_file) => write_to_file(transcripts, output_file, opts.force),
        None => write_to_stdout(transcripts),
    }
}

fn write_to_file<I>(merged: I, output_file: PathBuf, force: bool) -> Result<()>
where
    I: IntoIterator<Item = Transcript>,
{
    if output_file.exists() && !force {
        return Err(Error::OutputFileExists(output_file));
    }

    let file = File::create(output_file).map_err(Error::WriteError)?;
    write_merged_transcript(file, merged)?;
    Ok(())
}

fn write_to_stdout<I>(merged: I) -> Result<()>
where
    I: IntoIterator<Item = Transcript>,
{
    write_merged_transcript(std::io::stdout().lock(), merged)?;
    Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("no transcripts found for merging")]
    NoTranscripts,
    #[error("output file {0} exists, use --force to overwrite")]
    OutputFileExists(PathBuf),
    #[error("could not write to merged transcript: {0}")]
    WriteError(std::io::Error),
    #[error("could not load transcript: {0}")]
    TranscriptLoadFail(#[from] TranscriptError),
}

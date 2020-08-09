use argh::FromArgs;
use std::path::PathBuf;

/// Slice interviews and merge sliced F4 transcripts into a complete one.
#[derive(FromArgs)]
pub struct TopLevel {
    #[argh(subcommand)]
    pub invocation: Invocation,
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Invocation {
    Split(Split),
    Merge(Merge),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "merge")]
/// Merge multiple transcripts in F4 format and adjusts timestamps.
pub struct Merge {
    /// also merge F4 transcripts in subdirectories
    #[argh(switch, short = 'r')]
    pub recursive: bool,

    /// overwrite the output file if it exists
    #[argh(switch, short = 'f')]
    pub force: bool,

    /// list of files or directories
    #[argh(positional)]
    pub input_segments: Vec<PathBuf>,

    /// file to write the merged segment to, omit to write to standard output
    #[argh(option, short = 'o')]
    pub output_file: Option<PathBuf>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "split")]
/// Split interview sound files into enumerated 5min segments.
pub struct Split {
    /// one or more interviews to slice into 5min segments
    #[argh(positional)]
    pub input_files: Vec<PathBuf>,

    /// directory to write the split segments to, default to
    /// the same directory as the input files
    #[argh(option, short = 'o')]
    pub output_directory: Option<PathBuf>,

    /// also split sound files in subdirectories
    #[argh(switch, short = 'r')]
    pub recursive: bool,
}

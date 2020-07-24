use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Corrupt transcript: {0}")]
    Format(FormatError),
}

#[derive(Error, Debug)]
pub enum FormatError {
    /// Nothing in the transcript was recognized as a timestamp.
    /// We cannot do anything useful with a transcript like this.
    #[error("no timestamps found")]
    NoTimestampsFound,
    /// The transcript does not have the normal page setup code
    /// at the start that normally ends with `\jexpand`.
    #[error("malformed transcript RTF preamble")]
    MalformedPreamble,
    /// The transcript does not have the expected ending with
    /// a newline and a curly brace. It does not seem to have
    /// a format we understand and then we better not touch it.
    #[error("malformed transcript RTF epilogue")]
    MalformedEpilogue,
}

impl Error {
    pub fn no_timestamps_found() -> Self {
        Self::Format(FormatError::NoTimestampsFound)
    }

    pub fn malformed_preamble() -> Self {
        Self::Format(FormatError::MalformedPreamble)
    }

    pub fn malformed_epilogue() -> Self {
        Self::Format(FormatError::MalformedEpilogue)
    }
}

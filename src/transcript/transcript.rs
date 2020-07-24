use super::lines::Lines;

use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::fs::read_to_string;
use std::path::Path;

use crate::timestamp::Timestamp;
use crate::transcript::{Error, Result};

const PREAMBLE_END_PATTERN: &str = "\\jexpand\r\n";
const EPILOGUE: &str = "\r\n}";

#[derive(Clone)]
pub struct Transcript {
    // TODO just remember the offsets and read on demand for merging
    /// The preamble with RTF setup before the actual interview.
    preamble: String,
    /// The part of the transcript files that contains the actual
    /// transcript. It comes directly after the preamble and
    /// excludes the epilogue at the end that contains a newline and
    /// a `}`, which is the same for all transcripts.
    content: String,
    /// Suspected length of the interview segment, based on
    /// rounding up the last encountered timestamp.
    interview_end_time: Timestamp,
}

impl Transcript {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Transcript> {
        read_to_string(path)?.try_into()
    }

    /// The part of the transcript file before the main content,
    /// including the RTF header.
    pub fn preamble(&self) -> &str {
        &self.preamble
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// The part of the transript file after the main content,
    /// which closes the block that contains the main content.
    pub fn epilogue(&self) -> &str {
        EPILOGUE
    }

    pub fn lines(&self) -> Lines {
        Lines::new(self)
    }

    pub fn interview_end_time(&self) -> Timestamp {
        self.interview_end_time
    }
}

impl Display for Transcript {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{preamble}{content}{epilogue}",
            preamble = self.preamble(),
            content = self.content(),
            epilogue = EPILOGUE
        )
    }
}

impl TryFrom<String> for Transcript {
    type Error = Error;
    fn try_from(buf: String) -> Result<Transcript> {
        let content_start = find_content_start(&buf)?;
        let content_end = find_content_end(&buf)?;
        let preamble = String::from(&buf[0..content_start]);
        let content = String::from(&buf[content_start..content_end]);
        let interview_end_time = Timestamp::last_timestamp(&buf)
            .ok_or_else(Error::no_timestamps_found)?
            .round_up();
        Ok(Transcript {
            preamble,
            content,
            interview_end_time,
        })
    }
}

fn find_content_start(transcript: &str) -> Result<usize> {
    transcript
        .find(PREAMBLE_END_PATTERN)
        .map(|offset| offset + PREAMBLE_END_PATTERN.len())
        .ok_or_else(Error::malformed_preamble)
}

fn find_content_end(transcript: &str) -> Result<usize> {
    if transcript.ends_with(EPILOGUE) {
        Ok(transcript.len() - EPILOGUE.len())
    } else {
        Err(Error::malformed_epilogue())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn reference_transcript_01_is_wellformed() {
        let path = &Path::new("testdata/interview-01.rtf");
        assert!(Transcript::from_file(path).is_ok());
    }

    #[test]
    fn reference_transcript_02_is_wellformed() {
        let path = &Path::new("testdata/interview-02.rtf");
        assert!(Transcript::from_file(path).is_ok());
    }
}

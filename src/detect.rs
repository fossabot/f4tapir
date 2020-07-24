//! deals with detection of f4 files and sound files
use crate::timestamp::Timestamp;
use std::ffi::OsStr;
use std::io;
use std::path::Path;

/// Checks if the given path points to a file that appears to be
/// an F4 transcript.
///
/// We consider RTF files to be transcripts if they contain a string like
/// `#00:00:17-5#` in the first 4KiB. This is the default formatting of an
/// F4 time stamp, other formats are not supported (yet) in f4merge.
pub fn is_transcript(candidate: &Path) -> Result<bool, io::Error> {
    Ok(candidate.is_file()
        && has_rtf_extension(candidate)
        && Timestamp::contains_timestamps(candidate)?)
}

fn has_rtf_extension(candidate: &Path) -> bool {
    candidate.extension() == Some(OsStr::new("rtf"))
}

pub fn is_sound_file(file: &Path) -> bool {
    if let Some(ext) = file.extension() {
        if let Some(ext) = ext.to_str() {
            match ext {
                "mp3" | "MP3" | "wav" | "WAV" | "m4a" | "M4A" | "AAC" => true,
                _ => false,
            }
        } else {
            false
        }
    } else {
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn accept_reference_interview_01() {
        let path = &Path::new("testdata/interview-01.rtf");
        assert!(
            is_transcript(path).expect("Failed to check if reference interview is a transcript"),
            "Reference F4 transcript was not recognized as transcript"
        );
    }
}

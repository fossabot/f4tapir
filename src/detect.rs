//! deals with detection of f4 files and sound files
use crate::timestamp::Timestamp;
use std::ffi::OsStr;
use std::io;
use std::path::Path;

/// Lexicographically sorted list of accepted file endings.
///
/// Includes mostly audio formats, but also some video formats.
///
/// There are upper-case and lower-case versions for each.
const ACCEPTED_FILE_ENDINGS: [&'static str; 16] = [
    "3gp", "aac", "act", "amr", "avi", "flac", "m4a", "m4b", "mp3", "mp4", "oga", "oog", "vox",
    "wav", "webm", "wma",
];

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
            let ext: &str = &ext.to_ascii_lowercase();
            ACCEPTED_FILE_ENDINGS.binary_search(&ext).is_ok()
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
    use std::path::PathBuf;

    #[test]
    fn accept_reference_interview_01() {
        let path = &Path::new("testdata/interview-01.rtf");
        assert!(
            is_transcript(path).expect("Failed to check if reference interview is a transcript"),
            "Reference F4 transcript was not recognized as transcript"
        );
    }

    #[test]
    fn rtf_is_not_sound_file() {
        let path = &Path::new("testdata/interview-01.rtf");
        assert!(!is_sound_file(path));
    }

    #[test]
    fn accepted_file_endings_in_upper_case_are_recognized_as_sound_files() {
        for ending in &ACCEPTED_FILE_ENDINGS {
            let ending = ending.to_uppercase();
            let mut path = PathBuf::from("SOUND");
            path.set_extension(ending);
            assert!(
                is_sound_file(&path),
                "Expected the accepted file endings in upper case to be recognized as sound files, but {} was rejected", path.display()
            )
        }
    }

    #[test]
    fn accepted_file_endings_in_lower_case_are_recognized_as_sound_files() {
        for ending in &ACCEPTED_FILE_ENDINGS {
            let mut path = PathBuf::from("sound");
            path.set_extension(ending);
            assert!(
                is_sound_file(&path),
                "Expected the accepted file endings in lower case to be recognized as sound files, but {} was rejected", path.display()
            )
        }
    }

    #[test]
    fn accepted_file_endings_are_lexicogrpahically_sorted() {
        let endings = Vec::from(ACCEPTED_FILE_ENDINGS);
        let endings_sorted = {
            let mut sorted = endings.clone();
            sorted.sort();
            sorted
        };
        assert_eq!(
            endings, endings_sorted,
            "Accepted file endings to be lexicographically sorted"
        )
    }

    #[test]
    fn accepted_file_endings_have_no_duplicates() {
        let endings = Vec::from(ACCEPTED_FILE_ENDINGS);
        let endings_unique = {
            let mut sorted = endings.clone();
            sorted.dedup();
            sorted
        };
        assert_eq!(
            endings, endings_unique,
            "Accepted file endings might be sorted, but there are duplicate elements"
        )
    }
}

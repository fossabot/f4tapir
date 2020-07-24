//! Functionality to write one or more transcripts to a single,
//! file, adjusting the timestamps of later transcripts and
//! stitching together the last utterance of one transcript
//! with the first utterance of the next, if the speaker is
//! the same.
use super::lines::Line;
use std::io::Write;

use crate::timestamp::Timestamp;
use crate::transcript::{Result, Transcript};

/// Writes a merged version of the transcripts given with an
/// iterator to the given writable thing.
///
/// We try to stitch together adjacent transcripts if the
/// speakers are the same. We also adjust the timestamps.
///
/// If the transcript iterator is empty, does nothing and returns
/// an Ok result.
pub fn write_merged_transcript<W, I>(mut to: W, transcripts: I) -> Result<()>
where
    W: Write,
    I: IntoIterator<Item = Transcript>,
{
    let mut transcripts = transcripts.into_iter().peekable();
    let first_epilogue = {
        let first = match transcripts.peek() {
            Some(first) => first,
            None => return Ok(()),
        };
        write!(&mut to, "{}", first.preamble())?;
        first.epilogue().to_string()
    };

    let mut last_transcript = None;
    let mut shift = Timestamp::zero();
    for transcript in transcripts {
        let previous = last_transcript.as_ref().map(|t| (t, shift));
        let next_shift = shift
            + last_transcript
                .as_ref()
                .map(Transcript::interview_end_time)
                .unwrap_or_default();
        let next = (&transcript, next_shift);
        write_next_except_last_line(&mut to, previous, next)?;
        last_transcript = Some(transcript);
        shift = next_shift;
    }
    if let Some(last_transcript) = last_transcript {
        if let Some(last_line) = last_transcript.lines().next_back() {
            // write the excluded line from the last iteration
            last_line.write_adjusted(&mut to, shift)?;
        }
    }
    write!(&mut to, "{}", first_epilogue)?;
    Ok(())
}

/// Writes the lines of the first given transcript, assuming that the
/// content of the given `previous_transcript` has already been written.
///
/// If the last transcript ended with a line of the same speaker as the
/// frist line in the current transcript, we attempt to write these
/// lines in a merged way, that is, without the initial speaker label.
fn write_next_except_last_line<'a, W>(
    mut to: W,
    previous: Option<(&'a Transcript, Timestamp)>,
    current: (&'a Transcript, Timestamp),
) -> Result<()>
where
    W: Write,
{
    let (current_transcript, current_shift) = current;
    let mut lines = current_transcript.lines();

    // handle stitching with last transcript
    let previous_last_line_and_shift =
        previous
            .and_then(|(t, _)| t.lines().next_back())
            .map(|last_line| {
                let previous_shift = previous.map(|(_, ts)| ts).unwrap_or_default();
                (last_line, previous_shift)
            });

    match lines.next() {
        // we have a first line and maybe a last line too, try stitching
        Some(first_line) => {
            write_last_and_first_line(
                &mut to,
                previous_last_line_and_shift,
                first_line,
                current_shift,
            )?;
        }
        // not a single line in this transcript, write last line of last transcript and stop
        None => {
            if let Some((last_line, shift)) = previous_last_line_and_shift {
                last_line.write_adjusted(&mut to, shift)?;
            }
            return Ok(());
        }
    };

    // then continue writing everything except the last line
    let mut lines = lines.peekable();
    while let Some(line) = lines.next() {
        if lines.peek().is_none() {
            // last line, do not write and stop
            break;
        } else {
            line.write_adjusted(&mut to, current_shift)?;
        }
    }

    Ok(())
}

fn write_last_and_first_line<'a, W>(
    mut to: W,
    last_line_and_shift: Option<(Line<'a>, Timestamp)>,
    first_line: Line<'a>,
    shift: Timestamp,
) -> Result<()>
where
    W: Write,
{
    let previous_utterance_and_shift = last_line_and_shift
        .as_ref()
        .and_then(|(last_line, shift)| last_line.utterance().map(|u| (u, shift)));
    let first_utterance = first_line.utterance();
    match (previous_utterance_and_shift, first_utterance) {
        (Some((last, &last_shift)), Some(first)) if last.speaker() == first.speaker() => {
            // the last speaker from the last transcript and the first of this
            // transcripts are the same => do not duplicate the speaker label,
            // but merge the content of the utterances.
            last.write_adjusted_with_extra_speech(&mut to, last_shift, first.speech(), shift)?;
        }
        _ => {
            // different speakers or nothing to merge, one after the other or just one
            if let Some((last_line, last_shift)) = last_line_and_shift {
                last_line.write_adjusted(&mut to, last_shift)?;
            }
            first_line.write_adjusted(&mut to, shift)?;
        }
    };
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str;

    #[test]
    fn merge_001_and_002() {
        // given: transcripts that can be stitched
        let transcript001 = Transcript::from_file("testdata/interview-01.rtf")
            .expect("failed to load test transcript file");
        let transcript002 = Transcript::from_file("testdata/interview-02.rtf")
            .expect("failed to load test transcript file");

        // when: writing a stitched version to memory and getting the interesting line
        let mut buf = vec![];
        write_merged_transcript(&mut buf, vec![transcript001, transcript002])
            .expect("could not write merged transcipt");
        let merged = str::from_utf8(&buf[..]).expect("not valid utf-8");
        let mut merged_lines = merged.lines();
        let merged_line = merged_lines.nth(17).expect("could not get stitched line");
        let last_line = merged_lines.rev().nth(2).unwrap();

        // then
        assert_eq!(
            merged_line,
            "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Z:}\
            {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Ich glaub jetzt wei\\'df ich, worauf sie \
            hinauswollen. #00:04:50-3# Zunächst einmal ist der Punk nicht tot, ja? \
            #00:05:27-8# So auditiv meine ich. #00:05:31-6#}\\par}"
        );
        assert_eq!(
            last_line,
            "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Z:}\
            {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Ja, ja. #00:06:56-9#}\\par}"
        )
    }
}

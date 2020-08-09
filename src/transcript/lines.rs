//! Provides functionality to iterate over the utterances
//! in an interview.
//!
//! Each utterance has a speaker and some speech.
use std::convert::TryFrom;
use std::io::{Result, Write};

use super::rtf::{Rtf, TokenKind};

use crate::timestamp::Timestamp;
use crate::transcript::Transcript;

pub use paragraph::Paragraph;
pub use utterance::Utterance;

/// The canonical preamble to be used at the beginning of
/// every line, when it is written.
const LINE_PREAMBLE: &str = "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 ";
const LINE_EPILOGUE: &str = "\\par}";

/// Iterates over the utterances in a borrowed string
/// slice.
pub struct Lines<'a>(std::str::Lines<'a>);

impl<'a> Lines<'a> {
    /// Creates an iterator over the lines in the given
    /// transcript.
    pub fn new(source: &Transcript) -> Lines {
        Lines(source.content().lines())
    }

    fn parse_line(line: &'a str) -> Line<'a> {
        Self::trim_preamble_and_epilogue(line)
            .map(|line| match Utterance::try_from(line) {
                // ok, valid utterance
                Ok(utterance) => Line::Utterance(utterance),
                // also ok, a generic non-empty paragraph
                _ => Line::Paragraph(line.into()),
            })
            .unwrap_or_else(|| Line::Other(line))
    }

    fn trim_preamble_and_epilogue(line: &str) -> Option<&str> {
        let mut rtf = Rtf::from(line);

        rtf.next()
            .filter(|token| token.kind() == TokenKind::GroupStart)?;

        // \f0 or \f1
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\f")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        // \fs24
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\fs")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        // \ul0
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\ul")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        // \b0
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\b")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        // \i0
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\i")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        // \cf0
        rtf.next()
            .filter(|token| token.kind() == TokenKind::ControlWord)
            .filter(|token| token.as_str() == "\\cf")?;
        rtf.next()
            .filter(|token| token.kind() == TokenKind::Parameter)?;
        let last_token_of_preamble = rtf
            .next()
            .filter(|token| token.kind() == TokenKind::Delimiter)?;

        let content_start = last_token_of_preamble.source().end();
        let without_preamble = &line[content_start..];
        without_preamble.strip_suffix(LINE_EPILOGUE)
    }
}

#[derive(Debug)]
pub enum Line<'a> {
    Paragraph(Paragraph<'a>),
    Utterance(Utterance<'a>),
    Other(&'a str),
}

impl<'a> Line<'a> {
    pub fn write_adjusted<W>(&self, mut to: W, adjust_by: Timestamp) -> Result<()>
    where
        W: Write,
    {
        match self {
            // adjust timestamps in utterances and other paragraphs
            Self::Utterance(utterance) => utterance.write_adjusted(&mut to, adjust_by),
            Self::Paragraph(paragraph) => paragraph.write_adjusted(&mut to, adjust_by),
            // unrecognized RTF content, write as-is
            Self::Other(other) => write!(&mut to, "{}\r\n", other),
        }
    }

    pub fn utterance(&'a self) -> Option<&'a Utterance<'a>> {
        match self {
            Line::Utterance(utterance) => Some(utterance),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn paragraph(&'a self) -> Option<&'a Paragraph<'a>> {
        match self {
            Line::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    /// Uterance if could be parsed successfully, the offending
    /// line otherwise (excluding \r\n at the end of the line).
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(Self::parse_line)
    }
}

impl<'a> DoubleEndedIterator for Lines<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(Self::parse_line)
    }
}

mod paragraph {
    use super::{LINE_EPILOGUE, LINE_PREAMBLE};
    use crate::timestamp::Timestamp;
    use std::io::{Result, Write};

    /// A paragraph with non-utterance and non-blank content.
    ///
    /// The reference excludes a surrounding `{` on the
    /// left and a `\par}` on the right, holding only the
    /// actual content of the paragraph.
    #[derive(Debug)]
    pub struct Paragraph<'a> {
        content: &'a str,
    }

    impl<'a> Paragraph<'a> {
        pub fn write_adjusted<W>(&self, mut to: W, adjust_by: Timestamp) -> Result<()>
        where
            W: Write,
        {
            write!(&mut to, "{}", LINE_PREAMBLE)?;
            Timestamp::write_with_adjusted_timestamps(&mut to, self.content, adjust_by)?;
            write!(&mut to, "{}\r\n", LINE_EPILOGUE)?;
            Ok(())
        }

        #[cfg(test)]
        pub fn content(&self) -> &str {
            self.content
        }
    }

    impl<'a> From<&'a str> for Paragraph<'a> {
        fn from(content: &'a str) -> Self {
            Paragraph { content }
        }
    }
}

mod utterance {
    use std::convert::TryFrom;
    use std::io::{Result, Write};

    use super::{Rtf, LINE_EPILOGUE, LINE_PREAMBLE};
    use crate::timestamp::Timestamp;

    /// A paragraph that contains an utterance.
    ///
    /// The spearker is a short string (usually only one letter)
    /// for who supposedly said something. It is extracted from
    /// RTF code that occurs in slightly different variations at
    /// the beginning of every utterance, e.g. `Z` says something
    /// in this:
    /// ```
    /// {\f0 \fs24 \ul0 \b0 \i0 \cf0 {\f1 \fs24 \ul0 \b0 \i0 \cf0 Z:}{\f0 \fs24 \ul0 \b0 \i0 \cf0
    /// ```
    /// Sometimes transcripts use this form, with the colon at the end:
    /// ```
    /// {\f0 \fs24 \ul0 \b0 \i0 \cf0 {\f0 \fs24 \ul0 \b0 \i0 \cf0 Z}{\f0 \fs24 \ul0 \b0 \i0 \cf0 :
    /// ```
    /// Either way, the contained `speaker` reference contains only
    /// the code of the speaker, without the surrounding noise, which
    /// is instead stored in `speaker_before` and `speaker_after`.
    ///
    /// `speech` includes the main content after the colon and any
    /// closing curly brace directly after it. `}{\f0 \fs24 \ul0 \b0 \i0 \cf0`
    /// after the colon is also not included into the main speech.
    ///
    /// The utterance ends with `\par}\r\n` and athis is not included
    /// in any of the contained strings.
    ///
    /// All the strings are non-overlapping.
    #[derive(Debug)]
    pub struct Utterance<'a> {
        speaker_before: &'a str,
        speaker: &'a str,
        speaker_after: &'a str,
        speech: &'a str,
        speech_after: &'a str,
    }

    impl<'a> Utterance<'a> {
        pub fn speaker(&self) -> &str {
            self.speaker.trim()
        }

        pub fn speech(&self) -> &str {
            self.speech.trim()
        }

        pub fn write_adjusted<W>(&self, to: W, adjust_by: Timestamp) -> Result<()>
        where
            W: Write,
        {
            self.write_adjusted_with_extra_speech(to, adjust_by, "", Timestamp::zero())
        }

        /// Writes with adjusted timestamps and extra text with a different adjustement.
        pub fn write_adjusted_with_extra_speech<W>(
            &self,
            mut to: W,
            adjust_by: Timestamp,
            extra_speech: &str,
            extra_speech_adjust: Timestamp,
        ) -> Result<()>
        where
            W: Write,
        {
            write!(&mut to, "{}", LINE_PREAMBLE)?;
            write!(
                &mut to,
                "{}{}{}",
                self.speaker_before, self.speaker, self.speaker_after,
            )?;
            Timestamp::write_with_adjusted_timestamps(&mut to, self.speech.trim(), adjust_by)?;
            if !extra_speech.is_empty() {
                write!(&mut to, " ")?;
            }
            Timestamp::write_with_adjusted_timestamps(
                &mut to,
                extra_speech.trim(),
                extra_speech_adjust,
            )?;
            write!(&mut to, "{}", self.speech_after)?;
            write!(&mut to, "{}\r\n", LINE_EPILOGUE)?;
            Ok(())
        }
    }

    impl<'a> TryFrom<&'a str> for Utterance<'a> {
        type Error = ();

        /// Tries to convert a line into
        fn try_from(par: &'a str) -> std::result::Result<Self, Self::Error> {
            let mut text_content = Rtf::from(par).filter(|t| t.kind().is_text());

            let speaker = text_content.next().ok_or(())?;
            let speaker_src = speaker.source();
            let speaker_start = speaker_src.start();
            let speaker_end;
            let mut speech_start = None;
            let mut speech_end = None;

            if speaker.as_str().ends_with(':') && speaker.len() > 1 {
                // colon sticks right after the speaker name,
                // next node that follows is already the content
                speaker_end = Some(speaker_src.end() - 1);
                let speech = text_content.next().ok_or(())?;
                speech_start = Some(speech.source().start());
                speech_end = Some(speech.source().end());
            } else if speaker.as_str().contains(": ") {
                // speaker and speech are all in one node.
                speaker_end = speaker
                    .as_str()
                    .find(": ")
                    .map(|end| speaker_src.start() + end);
                speech_start = speaker_end.map(|e| e + ": ".len());
                speech_end = Some(speaker_src.end());
            } else {
                // colon is an extra text node or sticks before the content
                speaker_end = Some(speaker_src.end());
                let after_colon = text_content.next().ok_or(())?;
                if after_colon.as_str() == ":" || after_colon.as_str() == ": " {
                    // extra text node, content comes next
                    let speech = text_content.next().ok_or(())?;
                    speech_start = Some(speech.source().start());
                    speech_end = Some(speech.source().end());
                } else if after_colon.len() > 2 && after_colon.as_str().starts_with(": ") {
                    speech_start = Some(after_colon.source().start() + 2);
                    speech_end = Some(after_colon.source().end());
                }
            }

            // consume up to the last text block and extend speech_end
            if let Some(last_text) = text_content.last() {
                speech_end = Some(last_text.source().end());
            }

            let speaker_end = speaker_end.ok_or(())?;
            let speech_start = speech_start.ok_or(())?;
            let speech_end = speech_end.ok_or(())?;

            let speaker_before = &par[0..speaker_start];
            let speaker = &par[speaker_start..speaker_end];
            let speaker_after = &par[speaker_end..speech_start];
            let speech = &par[speech_start..speech_end];
            let speech_after = &par[speech_end..par.len()];

            Ok(Utterance {
                speaker_before,
                speaker,
                speaker_after,
                speech,
                speech_after,
            })
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn speaker_speech_in_one() {
            // given
            const UTTERANCE_RTF: &str =
                "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Z: Ich glaube auch, dass es nicht stimmt}";

            // when
            let utterance =
                Utterance::try_from(UTTERANCE_RTF).expect("could not parse as utterance");
            let speaker = utterance.speaker();
            let speech = utterance.speech();

            // then
            assert_eq!(speaker, "Z");
            assert_eq!(speech, "Ich glaube auch, dass es nicht stimmt");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;
    use std::hash::Hash;

    #[test]
    fn interview_01_odd_indexed_lines_are_empty_paragraphs() {
        // given: test transcript
        let transcript = Transcript::from_file("testdata/interview-01.rtf")
            .expect("failed to load test transcript file");
        let lines = transcript.lines();

        // when: getting the first and then every second line and assuming they are all empty paragraphs
        let odd_lines: Vec<_> = lines
            .enumerate()
            .filter_map(|(idx, line)| if (idx & 1) == 0 { Some(line) } else { None })
            .collect();
        let paragraph_contents : Vec<_> = odd_lines.iter().map(|l| l.paragraph().expect("expected all the odd lines to be empty paragraphs, but not all were paragraphs"))
            .map(Paragraph::content)
            .collect();

        // then: these should all be empty
        for paragraph_content in paragraph_contents {
            assert!(
                paragraph_content.is_empty(),
                "expected all the odd lines to be empty paragraphs, but not all were empty"
            );
        }
    }

    #[test]
    fn interview_01_even_indexed_lines_are_utterances() {
        // given: test transcript
        let transcript = Transcript::from_file("testdata/interview-01.rtf")
            .expect("failed to load test transcript file");
        let lines = transcript.lines();

        // when: getting the second line and then again every second line
        //       and assuming these are all utterances
        let even_lines: Vec<_> = lines
            .enumerate()
            .filter_map(|(idx, line)| if (idx & 1) == 1 { Some(line) } else { None })
            .collect();
        let utterances = even_lines.iter().map(|l| {
            l.utterance().expect(
                "expected all the odd lines to be empty paragraphs, but not all were paragraphs",
            )
        });
        let even_indexed_speakers: HashSet<&str> = utterances
            .clone()
            .enumerate()
            .filter_map(|(idx, line)| {
                if (idx & 1) == 0 {
                    Some(line.speaker())
                } else {
                    None
                }
            })
            .collect();
        let odd_indexed_speakers: HashSet<&str> = utterances
            .clone()
            .enumerate()
            .filter_map(|(idx, line)| {
                if (idx & 1) == 1 {
                    Some(line.speaker())
                } else {
                    None
                }
            })
            .collect();
        let speech: Vec<_> = utterances.clone().map(Utterance::speech).collect();

        // then: expect only speaker I on odd indexes,
        //       Z is speaker of the others,
        //       speech should be as expected
        assert_eq!(
            even_indexed_speakers,
            singleton("I"),
            "Expected I to speek on odd lines (even-indexed = odd-lined)"
        );
        assert_eq!(
            odd_indexed_speakers,
            singleton("Z"),
            "Expected Z to speek on even lines (odd-indexed = even-lined)"
        );
        assert_eq!(
            speech,
            vec![
                // umlaut ü is actually not supported in this RTF version, but we accept it for the sake of this test scenario
                "Was hat man fr\\'fcher so für Musik gehört #00:00:27-8#? So daheim und beim fortgehen meine ich. #00:00:31-6#",
                "Wir habens schon richtig hart krachen lassen bei den Punk-Konzerten damals #00:00:58-6#.",
                "War das da noch ein Ding? #00:00:58-9#",
                "Was soll das heißen? #00:01:50-6#",
                "Ja, so - #00:01:53-0# Der Punk ist halt tot, oder? #00:01:56-9#",
                "Ich glaub jetzt wei\\'df ich, worauf sie hinauswollen. #00:04:50-3#"
            ],
            "Speech was not parsed as expected"
        )
    }

    #[test]
    fn interview_02_even_indexed_lines_are_empty_paragraphs() {
        // given: test transcript 2
        let transcript = Transcript::from_file("testdata/interview-02.rtf")
            .expect("failed to load test transcript file");
        let lines = transcript.lines();

        // when: getting the second line and then again every second line
        //       and assuming these are all utterances
        let odd_lines: Vec<_> = lines
            .enumerate()
            .filter_map(|(idx, line)| if (idx & 1) == 1 { Some(line) } else { None })
            .collect();
        let paragraph_contents : Vec<_> = odd_lines.iter().map(|l| l.paragraph().expect("expected all the odd lines to be empty paragraphs, but not all were paragraphs"))
            .map(Paragraph::content)
            .collect();

        // then: these should all be empty
        for paragraph_content in paragraph_contents {
            assert!(
                paragraph_content.is_empty(),
                "expected all the odd lines to be empty paragraphs, but not all were empty"
            );
        }
    }

    #[test]
    fn interview_02_odd_indexed_lines_are_utterances() {
        // given: test transcript
        let transcript = Transcript::from_file("testdata/interview-02.rtf")
            .expect("failed to load test transcript file");
        let lines = transcript.lines();

        // when: getting the second line and then again every second line
        //       and assuming these are all utterances
        let even_lines: Vec<_> = lines
            .enumerate()
            .filter_map(|(idx, line)| if (idx & 1) == 0 { Some(line) } else { None })
            .collect();
        let utterances = even_lines.iter().map(|l| {
            l.utterance().expect(
                "expected all the odd lines to be empty paragraphs, but not all were paragraphs",
            )
        });
        let even_indexed_speakers: HashSet<&str> = utterances
            .clone()
            .enumerate()
            .filter_map(|(idx, line)| {
                if (idx & 1) == 0 {
                    Some(line.speaker())
                } else {
                    None
                }
            })
            .collect();
        let odd_indexed_speakers: HashSet<&str> = utterances
            .clone()
            .enumerate()
            .filter_map(|(idx, line)| {
                if (idx & 1) == 1 {
                    Some(line.speaker())
                } else {
                    None
                }
            })
            .collect();
        let speech: Vec<_> = utterances.clone().map(Utterance::speech).collect();

        // then: expect only speaker I on odd indexes,
        //       Z is speaker of the others,
        //       speech should be as expected
        assert_eq!(
            odd_indexed_speakers,
            singleton("I"),
            "Expected I to speek on even lines (odd-indexed = even-lined)"
        );
        assert_eq!(
            even_indexed_speakers,
            singleton("Z"),
            "Expected Z to speek on odd lines (even-indexed = odd-lined)"
        );
        assert_eq!(
            speech,
            vec![
                "Zunächst einmal ist der Punk nicht tot, ja? #00:00:27-8# So auditiv meine ich. #00:00:31-6#",
                "Versteh ich nicht #00:00:58-6#.",
                "Sie können denk Punk voll noch wahrnehmen, verstehen Sie? #00:00:58-9#",
                "Ich muss dann wieder los, ich hab noch was auf dem Herd. #00:01:50-6#",
                "Ja, ja. #00:01:56-9#",
            ],
            "Speech was not parsed as expected"
        );
    }

    fn singleton<T>(only_elem: T) -> HashSet<T>
    where
        T: Hash + Eq,
    {
        let mut hash = HashSet::new();
        hash.insert(only_elem);
        hash
    }

    #[test]
    fn alternative_preamble_is_accepted() {
        const LINE: &str = "{\\f1 \\fs24 \\ul0 \\b0 \\i0 \\cf0 {\\f1 \\fs24 \\ul0 \\b0 \\i0 \\cf0 I: Mhm. #00:03:10-1#}\\par}";
        let utterance = Lines::parse_line(LINE);
        assert!(
            utterance.utterance().is_some(),
            "Not an utterance: {:?}",
            utterance
        );
    }
}

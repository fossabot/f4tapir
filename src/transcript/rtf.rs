pub use token::{Token, TokenKind};

/// Double-ended iterator over RTF tokens in a string slice.
pub struct Rtf<'a> {
    source: &'a str,
    front_last_consumed: Option<TokenKind>,
    front_pos: usize,
    back_pos: usize,
}

impl<'a> Iterator for Rtf<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Token::parse(
            self.source,
            self.front_pos,
            self.back_pos,
            self.front_last_consumed,
        )
        .map(|token| {
            self.front_pos = token.source().end();
            self.front_last_consumed = Some(token.kind());
            token
        })
    }
}

impl<'a> From<&'a str> for Rtf<'a> {
    fn from(source: &'a str) -> Self {
        Rtf {
            source,
            front_pos: 0,
            back_pos: source.len(),
            front_last_consumed: None,
        }
    }
}

mod token {
    use std::fmt;

    #[derive(Clone, Copy, Debug)]
    pub struct Token<'a> {
        source: Extent<'a>,
        kind: TokenKind,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TokenKind {
        /// Unformatted text, including escape sequences like
        /// `\'fc`.
        Text,
        /// Start of an RTF group, indicated with a left curly
        /// brace `{`.
        GroupStart,
        /// End of an RTF group, indicated with a right curly
        /// brace `}`.
        GroupEnd,
        /// Similar to a control world, but has only a single
        /// nonalphabetic characer instead of an alphabetic
        /// string and also has no delimiter. E.g. `\~` is a
        /// non-breaking space.
        ControlSym,
        /// A control word, e.g. `\b0` would turn bold text
        /// off.
        ///
        /// They generally start with a backslash, followed by
        /// a lower-case letter sequence and some kind of
        /// delimiter.
        ///
        /// Spaces can be used as delimiters and are considered
        /// part of the control world.
        ///
        /// A digit or hyphen indicates that a numeric parameter
        /// with an arbitrary number of digits follows, e.g.
        /// `\green255`. The digit string is considered part
        /// of the control word.
        ///
        /// Any character other than a letter, hypthen or digit
        /// is also considered a delimiter, but is then not
        /// considered part of the control word.
        ///
        /// In any case, the delimiter is considered part of
        /// the control world and may play a role in the
        /// semantics of the control word.
        ControlWord,
        /// Optional integer parameter after a control word.
        Parameter,
        /// Delimiter that terminates a control symbol, control
        /// word or parameter, but is not part of the actual
        /// text content.
        Delimiter,
    }

    impl TokenKind {
        pub fn is_text(&self) -> bool {
            if let Self::Text = self {
                true
            } else {
                false
            }
        }
    }

    impl<'a> Token<'a> {
        /// Gets the extend of this token and the source code it originated
        /// from.
        pub fn source(&'a self) -> Extent<'a> {
            self.source
        }

        /// The kind of this token, can be used to
        /// check the type.
        pub fn kind(&self) -> TokenKind {
            self.kind
        }

        pub fn len(&self) -> usize {
            self.source.len()
        }

        pub fn parse(
            source: &'a str,
            from: usize,
            from_back: usize,
            last_consumed: Option<TokenKind>,
        ) -> Option<Self> {
            let len = from_back - from;
            let mut bytes = source.bytes().enumerate().skip(from).take(len);
            let (_, first_ch) = bytes.next()?;

            if let Some(TokenKind::ControlWord) = last_consumed {
                // control words may be followed by parameters
                if first_ch.is_ascii_digit() || first_ch == b'-' {
                    return Some(Self::parse_parameter(source, from));
                }
            }

            Some(match first_ch {
                b'\\' => Self::parse_control_word_or_symbol(source, from),
                b'{' => Self::new_group_start(source, from),
                b'}' => Self::new_group_end(source, from),
                _ => {
                    if let Some(TokenKind::ControlSym)
                    | Some(TokenKind::ControlWord)
                    | Some(TokenKind::Parameter) = last_consumed
                    {
                        // all of these have a separator after them, consume it without looking to much
                        // (there is a rule that spaces belong to the command before them, and other non-digit characters not, but who cares)
                        Self::new_delimiter(source, from)
                    } else {
                        Self::new_text(source, from, Self::consume_plain_text(source, from + 1))
                    }
                }
            })
        }

        /// Source code portion represented with this struct.
        pub fn as_str(&'a self) -> &'a str {
            self.source.as_str()
        }

        fn new_text(source: &'a str, from: usize, to: usize) -> Self {
            Self {
                source: Extent::new(source, from, to),
                kind: TokenKind::Text,
            }
        }

        fn new_control_sym(source: &'a str, at: usize) -> Self {
            Self {
                source: Extent::new(source, at, at + 2),
                kind: TokenKind::ControlSym,
            }
        }

        fn new_control_word(source: &'a str, from: usize, to: usize) -> Self {
            Self {
                source: Extent::new(source, from, to),
                kind: TokenKind::ControlWord,
            }
        }

        fn new_delimiter(source: &'a str, at: usize) -> Self {
            Self {
                source: Extent::new(source, at, at + 1),
                kind: TokenKind::Delimiter,
            }
        }

        fn new_group_start(source: &'a str, at: usize) -> Self {
            Self {
                source: Extent::new(source, at, at + 1),
                kind: TokenKind::GroupStart,
            }
        }

        fn new_group_end(source: &'a str, at: usize) -> Self {
            Self {
                source: Extent::new(source, at, at + 1),
                kind: TokenKind::GroupEnd,
            }
        }

        fn new_parameter(source: &'a str, from: usize, to: usize) -> Self {
            Self {
                source: Extent::new(source, from, to),
                kind: TokenKind::Parameter,
            }
        }

        fn parse_control_word_or_symbol(source: &'a str, from: usize) -> Self {
            match source.bytes().nth(from + 1) {
                // starts with character ecape sequence, treat as text and read on
                Some(b'\'') => {
                    Self::new_text(source, from, Self::consume_plain_text(source, from + 2))
                }
                Some(second_ch) if second_ch.is_ascii_lowercase() => {
                    Self::parse_control_word(source, from)
                }
                Some(_) => Self::new_control_sym(source, from),
                // Backslash at the end of the document. The spec does not really say if
                // this is valid, we just accept it as unformatted text.
                None => Self::new_text(source, from, from + 1),
            }
        }

        fn parse_parameter(source: &'a str, from: usize) -> Self {
            let param_end = source
                .bytes()
                .enumerate()
                // +1 => skip the first hyphen or digit, we know we are parsing a parameter
                .skip(from + 1)
                .skip_while(|(_, ch)| ch.is_ascii_digit())
                .map(|(idx, _)| idx)
                .next()
                .unwrap_or_else(|| source.len());
            Self::new_parameter(source, from, param_end)
        }

        fn parse_control_word(source: &'a str, from: usize) -> Self {
            let delimiter_pos = source
                .bytes()
                .enumerate()
                .skip(from)
                .skip(1) // skip the slash
                .find(|&(_, ch)| !ch.is_ascii_lowercase())
                .map(|(idx, _)| idx);

            Self::new_control_word(source, from, delimiter_pos.unwrap_or_else(|| source.len()))
        }

        /// Consumes plain text including escape sequences until the next
        /// proper control word.
        fn consume_plain_text(source: &'a str, from: usize) -> usize {
            let mut after_plain = source
                .bytes()
                .enumerate()
                .skip(from)
                // FIXME { escaping?
                .skip_while(|&(_, ch)| ch != b'\\' && ch != b'{' && ch != b'}');

            let first = after_plain.next().map(|(idx, _)| idx);
            let second = after_plain.next();
            match (first, second) {
                // escape sequence, read on as plain text and do not treat it
                // as a proper control word.
                (Some(_), Some((at, b'\''))) => Self::consume_plain_text(source, at + 1),
                // found next control word, end of plain text
                (Some(at), _) => at,
                // consumed all of the string
                _ => source.len(),
            }
        }
    }

    /// Holds a string slice and a beginning and ending
    /// index into that slice.
    ///
    /// Used to identify the position of tokens.
    #[derive(Clone, Copy)]
    pub struct Extent<'a> {
        source: &'a str,
        start: usize,
        end: usize,
    }

    impl<'a> fmt::Debug for Extent<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let source = &self.source[self.start..self.end];
            f.debug_struct("Source")
                .field("source()", &source)
                .field("len()", &self.len())
                .field("start", &self.start)
                .field("end", &self.end)
                .finish()
        }
    }

    impl<'a> Extent<'a> {
        pub fn new(source: &str, start: usize, end: usize) -> Extent {
            Extent { source, start, end }
        }

        /// Gets an extent for a given surrounding string and a
        /// portion of that string.
        ///
        /// # Panics
        /// Panics if the inner string is not really contained in
        /// the outer string.
        /*pub fn for_substring(outer: &'a str, inner: &'a str) -> Extent<'a> {
            let start = (inner.as_ptr() as usize) - (outer.as_ptr() as usize);
            let end = start + inner.len();
            assert!(
                start <= outer.len() && end <= outer.len(),
                "substring is not fully contained in outer string"
            );
            Extent {
                source: outer,
                start,
                end,
            }
        }*/

        /// Source code portion represented with this struct.
        pub fn as_str(&'a self) -> &'a str {
            &self.source[self.start..self.end]
        }

        /// Inclusive index of the first byte in this token,
        /// relative to the start of the RTF document.
        pub fn start(&self) -> usize {
            self.start
        }

        /// Exclusive index of the last byte in this token,
        /// relative to the start of the RTF document.
        pub fn end(&self) -> usize {
            self.end
        }

        /// Byte length of the token.
        pub fn len(&self) -> usize {
            self.end - self.start
        }
    }

    impl<'a> AsRef<str> for Extent<'a> {
        /// The string slice referenced by this source ref.
        fn as_ref(&self) -> &str {
            &self.source[self.start..self.end]
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn iterate_empty_paragraph() {
        const RTF_PARAGRAPH: &str = "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 \\par}";
        let rtf = Rtf::from(RTF_PARAGRAPH);
        let rtf_tokens: Vec<Token> = rtf.collect();
        let rtf_token_kinds: Vec<TokenKind> = rtf_tokens.iter().map(Token::kind).collect();
        let rtf_token_strings: Vec<&str> = rtf_tokens.iter().map(Token::as_str).collect();

        assert_eq!(
            rtf_token_strings,
            vec![
                "{", "\\f", "0", " ", "\\fs", "24", " ", "\\ul", "0", " ", "\\b", "0", " ", "\\i",
                "0", " ", "\\cf", "0", " ", "\\par", "}"
            ]
        );
        assert_eq!(
            rtf_token_kinds,
            vec![
                TokenKind::GroupStart,  // {
                TokenKind::ControlWord, // \f
                TokenKind::Parameter,   // 0
                TokenKind::Delimiter,   // " "
                TokenKind::ControlWord, // \fs
                TokenKind::Parameter,   // 24
                TokenKind::Delimiter,   // " "
                TokenKind::ControlWord, // \ul
                TokenKind::Parameter,   // 0
                TokenKind::Delimiter,   // " ",
                TokenKind::ControlWord, // \b
                TokenKind::Parameter,   // 0
                TokenKind::Delimiter,   // " ",
                TokenKind::ControlWord, // \i
                TokenKind::Parameter,   // 0
                TokenKind::Delimiter,   // " "
                TokenKind::ControlWord, // \cf
                TokenKind::Parameter,   // 0
                TokenKind::Delimiter,   // " "
                TokenKind::ControlWord, // \par
                TokenKind::GroupEnd     // }
            ]
        )
    }

    #[test]
    fn mhm_genau() {
        const RTF_WITH_TEXT: &str = "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 I}{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 : Mhm, genau. #00:00:19-0#}\\par}";
        let rtf_text_only: Vec<String> = Rtf::from(RTF_WITH_TEXT)
            .filter(|t| t.kind().is_text())
            .map(|token| token.as_str().to_string())
            .collect();
        assert_eq!(rtf_text_only, vec!["I", ": Mhm, genau. #00:00:19-0#"]);
    }

    #[test]
    fn zurueck_zu_den_methoden() {
        const RTF_WITH_TEXT: &str = "{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 {\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 Z}{\\f0 \\fs24 \\ul0 \\b0 \\i0 \\cf0 : Zur\\'fcck zu den Methoden #00:00:17-5#}\\par}";
        let rtf_text_only: Vec<String> = Rtf::from(RTF_WITH_TEXT)
            .filter(|t| t.kind().is_text())
            .map(|token| token.as_str().to_string())
            .collect();
        assert_eq!(
            rtf_text_only,
            vec!["Z", ": Zur\\'fcck zu den Methoden #00:00:17-5#"]
        );
    }
}

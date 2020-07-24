use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::io::{Read, Result as IOResult, Write};
use std::iter::Peekable;
use std::path::Path;

use thiserror::Error;

const F4_MAX_TIMESTAMP_LEN: usize = "#00:00:00-0#".len();

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u32);

impl Timestamp {
    const HOURS_LEN_MASK: u32 = 0b1100_0000_0000_0000_0000_0000_0000_0000;
    const HOURS_VAL_MASK: u32 = 0b0011_1111_1111_1100_0000_0000_0000_0000;
    const MINUTES_LEN_MASK: u32 = 0b0000_0000_0000_0010_0000_0000_0000_0000;
    const MINUTES_VAL_MASK: u32 = 0b0000_0000_0000_0001_1111_1000_0000_0000;
    const SECONDS_LEN_MASK: u32 = 0b0000_0000_0000_0000_0000_0100_0000_0000;
    const SECONDS_VAL_MASK: u32 = 0b0000_0000_0000_0000_0000_0011_1111_0000;
    // no len mask for sub-secs needed, always one digit
    const SUBSECS_VAL_MASK: u32 = 0b0000_0000_0000_0000_0000_0000_0000_1111;

    const HOURS_LEN_SHIFT: u32 = Self::HOURS_LEN_MASK.trailing_zeros();
    const HOURS_VAL_SHIFT: u32 = Self::HOURS_VAL_MASK.trailing_zeros();
    const MINUTES_LEN_SHIFT: u32 = Self::MINUTES_LEN_MASK.trailing_zeros();
    const MINUTES_VAL_SHIFT: u32 = Self::MINUTES_VAL_MASK.trailing_zeros();
    const SECONDS_LEN_SHIFT: u32 = Self::SECONDS_LEN_MASK.trailing_zeros();
    // no shift needed for subsecs, alredy all the way to the right
    const SECONDS_VAL_SHIFT: u32 = Self::SECONDS_VAL_MASK.trailing_zeros();
    // no shift for subsecs, which do not have a length

    const HOURS_LEN_MIN: u32 = 1;
    const HOURS_LEN_MAX: u32 = 4;
    const HOURS_VAL_MIN: u32 = 0;
    const HOURS_VAL_MAX: u32 = 4096 - 1;
    const MINUTES_LEN_MIN: u32 = 1;
    const MINUTES_LEN_MAX: u32 = 2;
    const MINUTES_VAL_MIN: u32 = 0;
    const MINUTES_VAL_MAX: u32 = 59;
    const SECONDS_LEN_MIN: u32 = 1;
    const SECONDS_LEN_MAX: u32 = 2;
    const SECONDS_VAL_MIN: u32 = 0;
    const SECONDS_VAL_MAX: u32 = 59;
    const SUBSECS_VAL_MAX: u32 = 9;

    fn new(hours: u32, minutes: u32, seconds: u32, subsecs: u32) -> Timestamp {
        let hours_len = if hours < 100 { 2 } else { 3 };
        Self::new_with_input_len(hours, hours_len, minutes, 2, seconds, 2, subsecs, 1)
    }

    pub fn zero() -> Timestamp {
        Self::new(0, 0, 0, 0)
    }

    fn new_with_input_len(
        hours: u32,
        hours_len: u32,
        minutes: u32,
        minutes_len: u32,
        seconds: u32,
        seconds_len: u32,
        subsecs: u32,
        subsecs_len: u32,
    ) -> Timestamp {
        assert!(
            subsecs_len == 1 && subsecs <= Self::SUBSECS_VAL_MAX,
            "subseconds out of bounds, expected one digit and max value 9"
        );
        Timestamp(
            // time: u32, time_mask: u32, time_shift: u32, time_min: u32, time_max: u32, len: u32, len_mask: u32, len_shift: u32, len_min: u32, len_max: u32
            Self::pack(
                hours,
                Self::HOURS_VAL_SHIFT,
                Self::HOURS_VAL_MIN,
                Self::HOURS_VAL_MAX,
                hours_len,
                Self::HOURS_LEN_SHIFT,
                Self::HOURS_LEN_MIN,
                Self::HOURS_LEN_MAX,
            ) | Self::pack(
                minutes,
                Self::MINUTES_VAL_SHIFT,
                Self::MINUTES_VAL_MIN,
                Self::MINUTES_VAL_MAX,
                minutes_len,
                Self::MINUTES_LEN_SHIFT,
                Self::MINUTES_LEN_MIN,
                Self::MINUTES_LEN_MAX,
            ) | Self::pack(
                seconds,
                Self::SECONDS_VAL_SHIFT,
                Self::SECONDS_VAL_MIN,
                Self::SECONDS_VAL_MAX,
                seconds_len,
                Self::SECONDS_LEN_SHIFT,
                Self::SECONDS_LEN_MIN,
                Self::SECONDS_LEN_MAX,
            ) | subsecs,
        )
    }

    pub fn extract_timestamps(buf: &[u8]) -> Vec<(usize, Timestamp)> {
        buf.as_ref()
            .windows(F4_MAX_TIMESTAMP_LEN)
            .enumerate()
            .filter_map(|(offset, window)| Timestamp::parse(window).map(|t| (offset, t)).ok())
            .collect()
    }

    /// Writes a version of the given input string slice with timestamps shifted
    /// by the specified value.
    ///
    /// Returns last adjusted timestamp that was written (if any).
    pub fn write_with_adjusted_timestamps<W>(
        mut to: W,
        content_with_timestamps: &str,
        by: Timestamp,
    ) -> IOResult<Option<Timestamp>>
    where
        W: Write,
    {
        let mut last_offset = 0;
        let timestamps = Timestamp::extract_timestamps(content_with_timestamps.as_ref());
        let mut last_adjusted = None;
        for &(after_ts_offset, after_ts_timestamp) in &timestamps {
            write!(
                &mut to,
                "{}",
                &content_with_timestamps[last_offset..after_ts_offset],
            )?;
            let adjusted = after_ts_timestamp + by;
            last_adjusted = Some(adjusted);
            write!(&mut to, "{}", adjusted)?;
            last_offset = after_ts_offset + after_ts_timestamp.len();
        }
        let after_last_timestamp =
            &content_with_timestamps[last_offset..content_with_timestamps.len()];
        write!(&mut to, "{}", after_last_timestamp)?;
        Ok(last_adjusted)
    }

    pub fn last_timestamp<B: AsRef<[u8]>>(buf: B) -> Option<Timestamp> {
        buf.as_ref()
            .windows(F4_MAX_TIMESTAMP_LEN)
            .filter_map(|window| Timestamp::parse(window).ok())
            .next_back()
    }

    pub fn contains_timestamps(candidate: &Path) -> IOResult<bool> {
        let mut file = File::open(candidate)?;
        let mut buf = [0_u8; 4096];
        let read_amount = file.read(&mut buf)?;

        for ts_window in (&buf[0..read_amount]).windows(F4_MAX_TIMESTAMP_LEN) {
            if Self::is_timestamp(ts_window) {
                // found something that looks like an F4 timestamp
                return Ok(true);
            }
        }

        Ok(false) // no obvious timestamp found in the first 4096 bytes
    }

    pub fn is_timestamp(timestamp_slice: &[u8]) -> bool {
        Self::parse(timestamp_slice).is_ok()
    }

    pub fn parse<S: AsRef<[u8]>>(timestamp: S) -> Result<Timestamp, Error> {
        Self::try_parse_timestamp(timestamp.as_ref()).ok_or_else(|| Error::malformed(timestamp))
    }

    fn try_parse_timestamp<S: AsRef<[u8]>>(timestamp: S) -> Option<Timestamp> {
        let timestamp = timestamp.as_ref();
        let mut bytes = timestamp.iter().cloned().peekable();

        expect_byte(&mut bytes, b'#')?;
        let (hours, hours_len) = parse_number(&mut bytes, Self::HOURS_VAL_MAX)?;
        expect_byte(&mut bytes, b':')?;
        let (minutes, minutes_len) = parse_number(&mut bytes, Self::MINUTES_VAL_MAX)?;
        expect_byte(&mut bytes, b':')?;
        let (seconds, seconds_len) = parse_number(&mut bytes, Self::SECONDS_VAL_MAX)?;
        expect_byte(&mut bytes, b'-')?;
        let (subsecs, subsecs_len) = parse_number(&mut bytes, Self::SUBSECS_VAL_MAX)?;
        expect_byte(&mut bytes, b'#')?;

        Some(Timestamp::new_with_input_len(
            hours,
            hours_len,
            minutes,
            minutes_len,
            seconds,
            seconds_len,
            subsecs,
            subsecs_len,
        ))
    }

    /// Rounds up the biggest unit and sets all the
    /// others to zero.
    ///
    /// Doing this on the last timestamp is likely the
    /// length of the interview segment.
    pub fn round_up(self) -> Timestamp {
        if self.hours() > 0 {
            if self.minutes() == 0 && self.seconds() == 0 && self.subsecs() == 0 {
                self
            } else {
                Timestamp::new(self.hours() + 1, 0, 0, 0)
            }
        } else if self.minutes() > 0 {
            if self.seconds() == 0 && self.subsecs() == 0 {
                self
            } else {
                Timestamp::new(0, self.minutes() + 1, 0, 0)
            }
        } else if self.seconds() > 0 {
            // only seconds, round up to one minute
            Timestamp::new(0, 1, 0, 0)
        } else {
            // only sub-seconds are just assumed at zero
            Timestamp::new(0, 0, 0, 0)
        }
    }

    pub fn hours(self) -> u32 {
        (self.0 & Self::HOURS_VAL_MASK) >> Self::HOURS_VAL_SHIFT
    }

    fn hours_len(self) -> u32 {
        Self::HOURS_LEN_MIN + ((self.0 & Self::HOURS_LEN_MASK) >> Self::HOURS_LEN_SHIFT)
    }

    pub fn minutes(self) -> u32 {
        (self.0 & Self::MINUTES_VAL_MASK) >> Self::MINUTES_VAL_SHIFT
    }

    fn minutes_len(self) -> u32 {
        Self::MINUTES_LEN_MIN + ((self.0 & Self::MINUTES_LEN_MASK) >> Self::MINUTES_LEN_SHIFT)
    }

    pub fn seconds(self) -> u32 {
        (self.0 & Self::SECONDS_VAL_MASK) >> Self::SECONDS_VAL_SHIFT
    }

    fn seconds_len(self) -> u32 {
        Self::SECONDS_LEN_MIN + ((self.0 & Self::SECONDS_LEN_MASK) >> Self::SECONDS_LEN_SHIFT)
    }

    pub fn subsecs(self) -> u32 {
        self.0 & Self::SUBSECS_VAL_MASK
    }

    fn subsecs_len(self) -> u32 {
        1
    }

    fn pack(
        time: u32,
        time_shift: u32,
        time_min: u32,
        time_max: u32,
        len: u32,
        len_shift: u32,
        len_min: u32,
        len_max: u32,
    ) -> u32 {
        assert!(
            time >= time_min && time <= time_max,
            "time out of bounds {}",
            time
        );
        assert!(
            len >= len_min && len <= len_max,
            "len out of bounds {}",
            len
        );
        let time = (time - time_min) << time_shift;
        let len = (len - len_min) << len_shift;
        time | len
    }

    /// Length of the timestamp when parsed.
    ///
    /// Accounts for missing leading zeroes and is really the original length.
    ///
    /// For timestamps created in code, gets the canonical length from
    /// `formatted_len`.
    pub fn len(self) -> usize {
        let len = 1 // #
        +
        self.hours_len()
        +
        1 // :
        +
        self.minutes_len() // minutes
        +
        1 // :
        +
        self.seconds_len() // seconds
        +
        1 // -
        +
        self.subsecs_len() // subseconds
        +
        1; // #
        len as usize
    }

    /// Disregards length form parsing and assumes the canonincal length that
    /// would be used when formatting the timestamp.
    #[cfg(test)]
    pub fn canonicalize_len(self) -> Timestamp {
        Timestamp::new(self.hours(), self.minutes(), self.seconds(), self.subsecs())
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp::zero()
    }
}

impl std::ops::Add for Timestamp {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let (subsecs, subsecs_carry) = carrying_add(self.subsecs(), rhs.subsecs(), 10);
        let (seconds, seconds_carry) =
            carrying_add(subsecs_carry + self.seconds(), rhs.seconds(), 60);
        let (minutes, minutes_carry) =
            carrying_add(seconds_carry + self.minutes(), rhs.minutes(), 60);
        let hours = minutes_carry + self.hours() + rhs.hours();
        Timestamp::new(hours, minutes, seconds, subsecs)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{hours:02}:{minutes:02}:{seconds:02}-{subsecs}#",
            hours = self.hours(),
            minutes = self.minutes(),
            seconds = self.seconds(),
            subsecs = self.subsecs()
        )
    }
}

impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Timestamp [ hours: {hours}, hours_len: {hours_len}, minutes: {minutes}, minutes_len: {minutes_len}, seconds: {seconds}, seconds_len: {seconds_len}, subsecs: {subsecs}, subsecs_len: {subsecs_len} ]",
            hours = self.hours(),
            minutes = self.minutes(),
            seconds = self.seconds(),
            subsecs = self.subsecs(),
            hours_len = self.hours_len(),
            minutes_len = self.minutes_len(),
            seconds_len = self.seconds_len(),
            subsecs_len = self.subsecs_len()
        )
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0} was not recognized as a timestamp")]
    Malformed(String),
}

impl Error {
    pub fn malformed<S: AsRef<[u8]>>(not_a_timestamp: S) -> Error {
        let not_a_timestamp = String::from_utf8_lossy(not_a_timestamp.as_ref()).to_string();
        Error::Malformed(not_a_timestamp)
    }
}

/// Parses one or more digits from the beginning of the given interator
/// and returns the number along with the digit count.
///
/// Returns `None` if no digits found.
fn parse_number<I>(bytes: &mut Peekable<I>, max: u32) -> Option<(u32, u32)>
where
    I: Iterator<Item = u8>,
{
    // need at least one leading number
    let mut number = bytes.next().and_then(parse_digit)? as u32;
    let mut digits = 1;

    let number = loop {
        match bytes.peek().cloned().and_then(parse_digit) {
            // found more ascii-digits, append as ones
            Some(next_digit) => {
                bytes.next().unwrap(); // consume the peeked digit
                digits += 1;
                number = number.checked_mul(10)?.checked_add(next_digit as u32)?;
            }
            // non-number or end of string found, stop and do not consume
            _ => break number,
        }
    };

    if number <= max {
        Some((number, digits))
    } else {
        None
    }
}

fn expect_byte<I>(bytes: &mut Peekable<I>, expect: u8) -> Option<()>
where
    I: Iterator<Item = u8>,
{
    bytes
        .next()
        .and_then(|b| if b == expect { Some(()) } else { None })
}

fn parse_digit(byte: u8) -> Option<u8> {
    if !byte.is_ascii_digit() {
        return None;
    }
    let digit = byte - b'0';
    Some(digit)
}

fn carrying_add(lhs: u32, rhs: u32, wrap_at: u32) -> (u32, u32) {
    let unwrapped_sum = lhs + rhs;
    let carry = unwrapped_sum / wrap_at;
    let sum = unwrapped_sum - carry * wrap_at;
    (sum, carry)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn recognize_normal_timestamp() {
        assert!(Timestamp::is_timestamp("#00:06:00-0#".as_bytes()))
    }

    #[test]
    fn reject_missing_colon() {
        assert!(!Timestamp::is_timestamp("#00:0600-0#".as_bytes()))
    }

    #[test]
    fn reject_point_instead_of_dash() {
        assert!(!Timestamp::is_timestamp("#00:06:00.0#".as_bytes()))
    }

    #[test]
    fn parsing() {
        // given
        let hours = 38;
        let minutes = 42;
        let seconds = 17;
        let subsecs = 9;
        let formatted = &format!(
            "#{hours}:{minutes}:{seconds}-{subsecs}#",
            hours = hours,
            minutes = minutes,
            seconds = seconds,
            subsecs = subsecs
        );

        // when
        let parsed = Timestamp::parse(formatted).expect("unexpected parse fail");

        // then
        assert_eq!(formatted, "#38:42:17-9#");
        assert_eq!(parsed.hours(), hours);
        assert_eq!(parsed.minutes(), minutes);
        assert_eq!(parsed.seconds(), seconds);
        assert_eq!(parsed.subsecs(), subsecs);
    }

    #[test]
    fn adjust_with_adding() {
        let a = Timestamp::parse("#58:58:57-9#").unwrap();
        let b = Timestamp::parse("#2:03:04-9#").unwrap();
        let a_plus_b = Timestamp::parse("#61:02:02-8#").unwrap();
        assert_eq!(
            a + b,
            a_plus_b,
            "\nExpected: {:?}\n        + {:?}\nto be:    {:?},\nbut was:  {:?}",
            a,
            b,
            a_plus_b,
            a + b
        )
    }

    #[test]
    fn round_up_at_hours() {
        let a = Timestamp::parse("#58:58:57-9#").unwrap();
        let rounded_up = Timestamp::parse("#59:00:00-0#").unwrap();
        assert_eq!(a.round_up(), rounded_up)
    }

    #[test]
    fn round_up_at_minutes() {
        let a = Timestamp::parse("#00:14:57-9#").unwrap();
        let rounded_up = Timestamp::parse("#00:15:00-0#").unwrap();
        assert_eq!(a.round_up(), rounded_up)
    }

    #[test]
    fn display() {
        let source = "#58:58:57-9#";
        let parsed = Timestamp::parse(source).unwrap();
        let formatted = format!("{}", parsed);
        assert_eq!(formatted, source);
    }

    #[test]
    fn allow_max_hours() {
        let source = format!("#{}:00:00-0#", Timestamp::HOURS_VAL_MAX);
        let parsed = Timestamp::parse(&source).unwrap();
        let formatted = format!("{}", parsed);
        assert_eq!(parsed.hours(), Timestamp::HOURS_VAL_MAX);
        assert_eq!(formatted, source);
    }

    #[test]
    #[should_panic]
    fn fail_parsing_out_of_bounds_hours() {
        let source = &format!("#{}:00:00-0#", Timestamp::HOURS_VAL_MAX + 1);
        Timestamp::parse(source).unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_parsing_out_of_bounds_minutes() {
        let source = "#00:60:00-0#";
        Timestamp::parse(source).unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_parsing_out_of_bounds_seconds() {
        let source = "#00:00:60-0#";
        Timestamp::parse(source).unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_parsing_out_of_bounds_subseconds() {
        let source = "#00:00:00-10#";
        Timestamp::parse(source).unwrap();
    }

    #[test]
    fn allow_one_digit_timestamp_components() {
        assert_eq!(
            Timestamp::parse("#1:2:3-4#").unwrap().canonicalize_len(),
            Timestamp::parse("#01:02:03-4#").unwrap()
        )
    }
}

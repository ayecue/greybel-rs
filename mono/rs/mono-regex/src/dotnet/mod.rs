mod ffi;

use std::time::Duration;

use ffi::{Output, Status, Utf16};

use super::{Match, RegexEngine, RegexError, RegexOptions};

const INFINITE_MATCH_TIMEOUT: i32 = -1;

#[derive(Debug, Clone, Copy, Default)]
pub struct DotnetRegexEngine {
    match_timeout: Option<i32>,
}

impl DotnetRegexEngine {
    #[must_use]
    pub fn new(match_timeout: Option<Duration>) -> Self {
        let milliseconds =
            |timeout: Duration| i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);

        Self {
            match_timeout: match_timeout.map(milliseconds),
        }
    }

    fn timeout(self) -> i32 {
        self.match_timeout.unwrap_or(INFINITE_MATCH_TIMEOUT)
    }
}

fn utf16(text: &str) -> Result<Utf16, RegexError> {
    Utf16::new(text).ok_or_else(|| RegexError::Failure("String too long.".into()))
}

fn to_result(output: Output) -> Result<Output, RegexError> {
    match output.status {
        Status::Ok => Ok(output),
        Status::Timeout => Err(RegexError::Timeout),
        Status::Failure => Err(RegexError::Failure(String::from_utf16_lossy(&output.text))),
    }
}

fn pieces(text: &[u16], lengths: impl IntoIterator<Item = i32>) -> impl Iterator<Item = String> {
    let mut remainder = text;

    lengths.into_iter().map(move |length| {
        let length = usize::try_from(length).unwrap_or(0).min(remainder.len());
        let (piece, after) = remainder.split_at(length);

        remainder = after;

        String::from_utf16_lossy(piece)
    })
}

impl RegexEngine for DotnetRegexEngine {
    fn is_match(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<bool, RegexError> {
        let output = ffi::is_match(
            &utf16(input)?,
            &utf16(pattern)?,
            options.bits(),
            self.timeout(),
        );

        to_result(output).map(|output| output.is_match)
    }

    fn matches(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<Match>, RegexError> {
        let output = to_result(ffi::matches(
            &utf16(input)?,
            &utf16(pattern)?,
            options.bits(),
            self.timeout(),
        ))?;
        let (indices, lengths): (Vec<i32>, Vec<i32>) = output
            .numbers
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[index, length]| (index, length))
            .unzip();

        Ok(indices
            .into_iter()
            .zip(pieces(&output.text, lengths))
            .map(|(index, value)| Match {
                index: usize::try_from(index).unwrap_or(0),
                value,
            })
            .collect())
    }

    fn split(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<String>, RegexError> {
        let output = to_result(ffi::split(
            &utf16(input)?,
            &utf16(pattern)?,
            options.bits(),
            self.timeout(),
        ))?;

        Ok(pieces(&output.text, output.numbers.iter().copied()).collect())
    }

    fn replace(
        &self,
        input: &str,
        pattern: &str,
        replacement: &str,
        options: RegexOptions,
    ) -> Result<String, RegexError> {
        let output = to_result(ffi::replace(
            &utf16(input)?,
            &utf16(pattern)?,
            &utf16(replacement)?,
            options.bits(),
            self.timeout(),
        ))?;

        Ok(String::from_utf16_lossy(&output.text))
    }
}

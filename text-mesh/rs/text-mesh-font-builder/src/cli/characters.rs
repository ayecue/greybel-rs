use super::error::CliError;

/// Ordered characters from literal text and hexadecimal ranges such as `20-7E` or `U+2500-U+257F`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterList {
    characters: Vec<char>,
}

impl CharacterList {
    pub fn push_text(&mut self, text: &str) {
        self.characters.extend(
            text.chars()
                .filter(|character| *character != '\n' && *character != '\r'),
        );
    }

    /// # Errors
    /// Returns an error for invalid hexadecimal code points or characters in the range.
    pub fn push_range(&mut self, range: &str) -> Result<(), CliError> {
        let range = range.trim();

        if range.is_empty() {
            return Ok(());
        }

        let (first, last) = range.split_once('-').unwrap_or((range, range));
        let (first, last) = (Self::code_point(first)?, Self::code_point(last)?);

        if first > last {
            return Err(CliError::Range(range.to_owned()));
        }

        self.characters
            .extend((first..=last).filter_map(char::from_u32));

        Ok(())
    }

    fn code_point(text: &str) -> Result<u32, CliError> {
        let text = text.trim();
        let digits = text
            .strip_prefix("U+")
            .or_else(|| text.strip_prefix("u+"))
            .or_else(|| text.strip_prefix("0x"))
            .unwrap_or(text);

        u32::from_str_radix(digits, 16).map_err(|_| CliError::Range(text.to_owned()))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.characters.is_empty()
    }

    #[must_use]
    pub fn into_characters(self) -> Vec<char> {
        self.characters
    }
}

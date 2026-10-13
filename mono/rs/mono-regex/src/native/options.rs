use super::cursor::Cursor;
use crate::RegexOptions;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub ignore_case: bool,
    pub multiline: bool,
    pub singleline: bool,
    pub explicit_capture: bool,
    pub ignore_whitespace: bool,
}

impl From<RegexOptions> for Options {
    fn from(options: RegexOptions) -> Self {
        Self {
            ignore_case: options.contains(RegexOptions::IGNORE_CASE),
            multiline: options.contains(RegexOptions::MULTILINE),
            singleline: options.contains(RegexOptions::SINGLELINE),
            explicit_capture: options.contains(RegexOptions::EXPLICIT_CAPTURE),
            ignore_whitespace: options.contains(RegexOptions::IGNORE_PATTERN_WHITESPACE),
        }
    }
}

impl Options {
    pub fn scan(&mut self, cursor: &mut Cursor<'_>) {
        let mut on = true;

        while let Some(c) = cursor.peek() {
            match c {
                '-' => on = false,
                '+' => on = true,
                c if self.set(c, on) => {}
                _ => return,
            }

            cursor.bump();
        }
    }

    fn set(&mut self, letter: char, on: bool) -> bool {
        let option = match letter.to_ascii_lowercase() {
            'i' => &mut self.ignore_case,
            'm' => &mut self.multiline,
            's' => &mut self.singleline,
            'n' => &mut self.explicit_capture,
            'x' => &mut self.ignore_whitespace,
            _ => return false,
        };

        *option = on;

        true
    }
}

/// How the terminal splits a long output line into lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineSplitRule {
    /// The game's rule since 0.9.5683: a line longer than 256 UTF-16 units is cut into pieces of 180, or 900 when
    /// mostly tags, unless its visible text is at most 256 units and its tags at most 2048.
    #[default]
    Game,
    /// greyhack-customizable-output's rule: a line whose visible text is longer than `text_per_line` units, or
    /// whose tags take more than `tags_per_line` units, is cut into pieces of `text_per_line` units.
    Limits(LineLimits),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineLimits {
    /// At least 1.
    pub text_per_line: usize,
    /// `None` for no limit.
    pub tags_per_line: Option<usize>,
}

impl LineLimits {
    /// greyhack-customizable-output's defaults, the limits before 0.9.5683: 256 units of text, any number of tags.
    pub const BEFORE_TAG_LIMIT: Self = Self {
        text_per_line: 256,
        tags_per_line: None,
    };

    /// Raises `text_per_line` to at least 1.
    #[must_use]
    pub fn new(text_per_line: usize, tags_per_line: Option<usize>) -> Self {
        Self {
            text_per_line: text_per_line.max(1),
            tags_per_line,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalOutput<'a> {
    text: &'a str,
    split_rule: LineSplitRule,
}

impl<'a> TerminalOutput<'a> {
    const CHUNK: usize = 180;
    const LONG_LINE: usize = 256;
    const TAG_HEAVY_CHUNK: usize = 900;
    const TAG_HEAVY_UNITS: usize = 2048;

    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self {
            text,
            split_rule: LineSplitRule::Game,
        }
    }

    #[must_use]
    pub const fn with_split_rule(self, split_rule: LineSplitRule) -> Self {
        Self { split_rule, ..self }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let lf_text = self.text.replace("\r\n", "\n").replace('\r', "\n");
        let mut lines = Vec::new();

        for line in lf_text.split('\n') {
            match self.split_rule {
                LineSplitRule::Game => Self::push_line(line, &mut lines),
                LineSplitRule::Limits(limits) => Self::push_limited_line(line, limits, &mut lines),
            }
        }

        lines
    }

    fn push_line(line: &str, lines: &mut Vec<String>) {
        let units: Vec<u16> = line.encode_utf16().collect();
        let length = units.len();

        if length <= Self::LONG_LINE {
            lines.push(line.to_owned());

            return;
        }

        let visible_length = Self::without_tags(line);
        let tag_heavy = length - visible_length > Self::TAG_HEAVY_UNITS;

        if visible_length <= Self::LONG_LINE && !tag_heavy {
            lines.push(line.to_owned());

            return;
        }

        let chunk = if !tag_heavy && visible_length < length / 2 {
            Self::TAG_HEAVY_CHUNK
        } else {
            Self::CHUNK
        };

        lines.extend(units.chunks(chunk).map(String::from_utf16_lossy));
    }

    fn push_limited_line(line: &str, limits: LineLimits, lines: &mut Vec<String>) {
        let units: Vec<u16> = line.encode_utf16().collect();
        let visible_length = Self::without_tags(line);
        let tag_length = units.len() - visible_length;
        let exceeds_text = visible_length > limits.text_per_line;
        let exceeds_tags = limits.tags_per_line.is_some_and(|limit| tag_length > limit);

        if !exceeds_text && !exceeds_tags {
            lines.push(line.to_owned());

            return;
        }

        lines.extend(
            units
                .chunks(limits.text_per_line.max(1))
                .map(String::from_utf16_lossy),
        );
    }

    fn without_tags(line: &str) -> usize {
        let mut length = 0;
        let mut remainder = line;

        while let Some(open) = remainder.find('<') {
            length += remainder[..open].encode_utf16().count();

            match remainder[open + 1..].find('>') {
                Some(close) => remainder = &remainder[open + 1 + close + 1..],
                None => {
                    length += remainder[open..].encode_utf16().count();

                    return length;
                }
            }
        }

        length + remainder.encode_utf16().count()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalOutput<'a> {
    text: &'a str,
}

impl<'a> TerminalOutput<'a> {
    const CHUNK: usize = 180;
    const LONG_LINE: usize = 256;
    const TAG_HEAVY_CHUNK: usize = 900;
    const TAG_HEAVY_UNITS: usize = 2048;

    #[must_use]
    pub const fn new(text: &'a str) -> Self {
        Self { text }
    }

    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let lf_text = self.text.replace("\r\n", "\n").replace('\r', "\n");
        let mut lines = Vec::new();

        for line in lf_text.split('\n') {
            Self::push_line(line, &mut lines);
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

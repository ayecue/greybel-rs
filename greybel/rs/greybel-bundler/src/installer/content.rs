use std::ops::Range;

#[derive(Debug, Clone)]
pub(crate) struct InstallerContent {
    text: String,
    escapes: Vec<Range<usize>>,
}

impl InstallerContent {
    const IMPORT_CODE: &str = "import_code(";

    pub(crate) fn new(code: &str) -> Self {
        let mut text = String::with_capacity(code.len() + code.len() / 8);
        let mut escapes = Vec::new();
        let mut remainder = code;

        while let Some(character) = remainder.chars().next() {
            let start = text.len();

            if remainder.len() >= Self::IMPORT_CODE.len()
                && remainder.as_bytes()[..Self::IMPORT_CODE.len()]
                    .eq_ignore_ascii_case(Self::IMPORT_CODE.as_bytes())
            {
                let call = &remainder[..Self::IMPORT_CODE.len()];

                text.push_str(&call[..6]);
                text.push_str("\"+\"");
                text.push_str(&call[6..7]);
                text.push_str("\"+\"");
                text.push_str(&call[7..]);
                escapes.push(start..text.len());
                remainder = &remainder[Self::IMPORT_CODE.len()..];
                continue;
            }

            if character == '"' {
                text.push_str("\"\"");
                escapes.push(start..text.len());
            } else {
                text.push(character);
            }

            remainder = &remainder[character.len_utf8()..];
        }

        Self { text, escapes }
    }

    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }

    pub(crate) fn slice(&self, range: Range<usize>) -> &str {
        &self.text[range]
    }

    pub(crate) fn cut(&self, from: usize, budget: usize) -> usize {
        let mut end = from;
        let mut utf16_len = 0;

        for character in self.text[from..].chars() {
            utf16_len += character.len_utf16();
            if utf16_len > budget {
                break;
            }

            end += character.len_utf8();
        }

        let split = self.escapes.partition_point(|escape| escape.start < end);

        if let Some(escape) = split.checked_sub(1).map(|index| &self.escapes[index])
            && escape.start < end
            && end < escape.end
        {
            end = escape.start.max(from);
        }

        end
    }
}

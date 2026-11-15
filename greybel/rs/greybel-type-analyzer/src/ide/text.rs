#[derive(Clone, Copy)]
pub(crate) struct SourceText<'a> {
    source: &'a str,
}

impl<'a> SourceText<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self { source }
    }

    fn is_name(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    pub(crate) fn name_start(self, end: usize) -> usize {
        self.source[..end]
            .char_indices()
            .rev()
            .take_while(|&(_, c)| Self::is_name(c))
            .last()
            .map_or(end, |(start, _)| start)
    }

    pub(crate) fn member_chain_before(self, end: usize) -> Option<Vec<String>> {
        let mut names = Vec::new();
        let mut end = end;

        loop {
            let start = self.name_start(end);

            if start == end {
                return None;
            }

            names.push(self.source[start..end].to_owned());

            let before = self.source[..start].chars().next_back();

            if before != Some('.') {
                let starts_name =
                    before.is_none_or(|c| !Self::is_name(c) && !matches!(c, ')' | ']' | '"'));

                names.reverse();

                return starts_name.then_some(names);
            }

            end = start - 1;
        }
    }

    pub(crate) fn open_call(self, offset: usize) -> Option<(usize, usize)> {
        let line_start = self.source[..offset]
            .rfind(['\n', '\r'])
            .map_or(0, |at| at + 1);
        let text = &self.source[line_start..offset];
        let mut depth = 0usize;
        let mut active_parameter = 0usize;
        let mut in_string = false;
        let mut open_paren = None;

        for (at, c) in text.char_indices().rev() {
            if c == '"' {
                in_string = !in_string;
                continue;
            }

            if in_string {
                continue;
            }

            match c {
                ')' | ']' | '}' => depth += 1,
                '[' | '{' | '(' if depth > 0 => depth -= 1,
                '[' | '{' => return None,
                '(' => {
                    open_paren = Some(at);
                    break;
                }
                ',' if depth == 0 => active_parameter += 1,
                _ => {}
            }
        }

        let open_paren = line_start + open_paren?;
        let callee_end = self.source[..open_paren].trim_end().len();

        Some((callee_end, active_parameter))
    }
}

#[cfg(test)]
#[path = "tests/text.rs"]
mod tests;

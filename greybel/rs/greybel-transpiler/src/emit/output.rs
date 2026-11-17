use std::fmt;

use greybel_documents::ModuleId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    pub module: ModuleId,
    pub line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EmitOutput {
    code: String,
    starts: Vec<usize>,
    line_start: usize,
    comments: Vec<String>,
    origins: Vec<Option<Origin>>,
    origin: Option<Origin>,
}

impl EmitOutput {
    #[must_use]
    pub fn with_capacity(code: usize) -> Self {
        Self {
            code: String::with_capacity(code),
            ..Self::default()
        }
    }

    pub fn push(&mut self, text: &str) {
        self.code.push_str(text);
    }

    pub fn push_fmt(&mut self, args: fmt::Arguments<'_>) {
        fmt::Write::write_fmt(&mut self.code, args).expect("a String accepts any write");
    }

    pub fn mark(&mut self, origin: Origin) {
        self.origin.get_or_insert(origin);
    }

    #[must_use]
    pub fn origins(&self) -> &[Option<Origin>] {
        &self.origins
    }

    pub fn eol(&mut self) {
        self.origins.push(self.origin.take());
        self.starts.push(self.line_start);
        self.code.push('\n');
        self.line_start = self.code.len();
    }

    #[must_use]
    pub fn is_line_empty(&self) -> bool {
        self.code.len() == self.line_start
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    pub fn append_comment(&mut self, index: usize, comment: impl fmt::Display) {
        if self.comments.len() <= index {
            self.comments.resize_with(index + 1, String::new);
        }

        let line_comments = &mut self.comments[index];

        if !line_comments.is_empty() {
            line_comments.push(' ');
        }

        fmt::Write::write_fmt(line_comments, format_args!("{comment}"))
            .expect("a String accepts any write");
    }

    #[must_use]
    pub fn text_len(&self) -> usize {
        self.code.len()
            + self
                .comments
                .iter()
                .map(|comment| comment.len() + 1)
                .sum::<usize>()
    }

    #[must_use]
    pub fn into_code(mut self) -> String {
        self.code.truncate(self.line_start.saturating_sub(1));

        self.code
    }

    pub fn lines(&self) -> impl Iterator<Item = (&str, &str)> {
        let ends = self.starts.iter().skip(1).map(|&start| start - 1).chain(
            std::iter::once(self.line_start.saturating_sub(1)).filter(|_| !self.starts.is_empty()),
        );

        self.starts
            .iter()
            .zip(ends)
            .enumerate()
            .map(|(index, (&start, end))| {
                let comment = self.comments.get(index).map_or("", String::as_str);

                (&self.code[start..end], comment)
            })
    }
}

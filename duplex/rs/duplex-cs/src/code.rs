//! Indented C# source builder.

const INDENT: &str = "    ";

/// C# source text and current indentation depth.
#[derive(Debug, Default)]
pub(crate) struct Code {
    text: String,
    depth: usize,
}

impl Code {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        let line = line.as_ref();

        if !line.is_empty() {
            for _ in 0..self.depth {
                self.text.push_str(INDENT);
            }

            self.text.push_str(line);
        }

        self.text.push('\n');
    }

    /// Writes `head` and a braced block whose lines are emitted by `body`.
    pub(crate) fn block(&mut self, head: impl AsRef<str>, body: impl FnOnce(&mut Self)) {
        self.line(head);
        self.line("{");
        self.depth += 1;
        body(self);
        self.depth -= 1;
        self.line("}");
    }

    /// Writes nonempty `lines` as an XML-escaped `<summary>` block.
    pub(crate) fn docs(&mut self, lines: &[String]) {
        if lines.is_empty() {
            return;
        }

        self.line("/// <summary>");
        for line in lines {
            let mut text = String::new();

            for char in line.chars() {
                match char {
                    '&' => text.push_str("&amp;"),
                    '<' => text.push_str("&lt;"),
                    '>' => text.push_str("&gt;"),
                    char => text.push(char),
                }
            }

            self.line(format!("/// {}", text.trim_end()));
        }

        self.line("/// </summary>");
    }

    pub(crate) fn finish(self) -> String {
        self.text
    }
}

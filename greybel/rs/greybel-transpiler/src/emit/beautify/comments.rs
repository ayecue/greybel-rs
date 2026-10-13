use std::fmt;

use greybel_core::{NodeId, Span};
use greybel_symbols::delimiters::LINE_COMMENT;

use super::BeautifyEmitter;
use crate::emit::EmitState;

impl BeautifyEmitter {
    pub(super) fn emit_leading_comments(&self, s: &mut EmitState<'_>, id: NodeId) {
        let Some(comments) = s.comments(id) else {
            return;
        };

        for &comment in &comments.lead_comments {
            let text = CommentText::new(s.slice(comment), s.options().dev_mode);

            s.push_fmt(format_args!("{}{text}", self.indentation()));
            s.eol();
        }
    }

    pub(super) fn emit_trailing_comments(s: &mut EmitState<'_>, id: NodeId, line: Option<usize>) {
        let Some(comments) = s.comments(id) else {
            return;
        };

        Self::append_comments(s, &comments.trailing_comments, line);
    }

    pub(super) fn emit_end_trailing_comments(s: &mut EmitState<'_>, id: NodeId) {
        let Some(comments) = s.comments(id) else {
            return;
        };

        Self::append_comments(s, &comments.end_trailing_comments, None);
    }

    pub(super) fn append_comments(s: &mut EmitState<'_>, comments: &[Span], line: Option<usize>) {
        let dev_mode = s.options().dev_mode;

        for &comment in comments {
            s.append_comment(line, CommentText::new(s.slice(comment), dev_mode));
        }
    }

    pub(super) fn emit_blank_line(&self, s: &mut EmitState<'_>, noop: NodeId) {
        if s.comments(noop)
            .is_some_and(|comments| !comments.lead_comments.is_empty())
        {
            self.emit_leading_comments(s, noop);
        } else {
            self.push_indent(s);
            s.eol();
        }
    }
}

struct CommentText<'a> {
    value: &'a str,
    dev_mode: bool,
}

impl<'a> CommentText<'a> {
    fn new(value: &'a str, dev_mode: bool) -> Self {
        Self { value, dev_mode }
    }

    fn line(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
        if text.is_empty() {
            f.write_str(LINE_COMMENT)
        } else {
            write!(f, "{LINE_COMMENT} {text}")
        }
    }
}

impl fmt::Display for CommentText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = self.value.trim();

        if !self.value.contains('\n') {
            return Self::line(f, body);
        }

        if self.dev_mode {
            return write!(f, "/* {body} */");
        }

        for (index, text) in body.split('\n').enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }

            Self::line(f, text.trim())?;
        }

        Ok(())
    }
}

use oxc_ast::ast::CommentNewlines;
use oxc_ast::{Comment, CommentContent, CommentKind, CommentPosition};
use oxc_span::{SPAN, Span};

const LINE_START: &str = "//";
const DOC_START: &str = "/**";
const DOC_LINE: &str = " *";
const DOC_END: &str = " */";

#[derive(Default)]
pub(crate) struct CommentStorage {
    text: String,
    comments: Vec<Comment>,
}

impl CommentStorage {
    pub(crate) fn header(&mut self, line: &str) {
        let start = self.text.len();

        self.text.push_str(LINE_START);
        self.text.push(' ');
        self.text.push_str(line);
        self.push(start, CommentKind::Line, CommentContent::None, SPAN.start);
    }

    pub(crate) fn document(&mut self, lines: &[String]) -> Span {
        if lines.is_empty() {
            return SPAN;
        }

        let start = self.text.len();

        match lines {
            [line] => {
                self.text.push_str(DOC_START);
                self.text.push(' ');
                self.text.push_str(line);
                self.text.push_str(DOC_END);
            }
            lines => {
                self.text.push_str(DOC_START);
                for line in lines {
                    self.text.push('\n');
                    self.text.push_str(DOC_LINE);
                    if !line.is_empty() {
                        self.text.push(' ');
                        self.text.push_str(line);
                    }
                }

                self.text.push('\n');
                self.text.push_str(DOC_END);
            }
        }

        let kind = if lines.len() == 1 {
            CommentKind::SingleLineBlock
        } else {
            CommentKind::MultiLineBlock
        };
        let attach_offset = Self::offset(self.text.len()) + 1;

        self.push(start, kind, CommentContent::Jsdoc, attach_offset);

        Span::new(attach_offset, attach_offset)
    }

    fn push(
        &mut self,
        start: usize,
        kind: CommentKind,
        content: CommentContent,
        attach_offset: u32,
    ) {
        let mut comment = Comment::new(Self::offset(start), Self::offset(self.text.len()), kind);

        comment.attached_to = attach_offset;
        comment.position = CommentPosition::Leading;
        comment.newlines = CommentNewlines::Leading | CommentNewlines::Trailing;
        comment.content = content;
        self.comments.push(comment);
        self.text.push('\n');
    }

    fn offset(index: usize) -> u32 {
        u32::try_from(index).expect("the comments of a module fit in 4 GiB")
    }

    pub(crate) fn into_parts(self) -> (String, Vec<Comment>) {
        (self.text, self.comments)
    }
}

//! Emits a [`Node`] tree as `fancy-regex` syntax, wrapping every node except concatenations and alternations so quantifiers can follow.

use std::fmt::Write as _;

use super::ast::{Assertion, Class, ClassItem, Condition, GroupKind, Matcher, Node, Set, SetKind};
use super::cursor::WORD_MEMBERS;
use super::error::Reason;
use super::groups::GroupTable;

pub fn emit(node: &Node, groups: &GroupTable, ignore_case: bool) -> Result<String, Reason> {
    let mut emitter = Emitter {
        out: String::new(),
        groups,
        ignore_case,
    };

    if ignore_case {
        emitter.out.push_str("(?i)");
    }

    emitter.node(node)?;

    Ok(emitter.out)
}

struct Emitter<'g> {
    out: String,
    groups: &'g GroupTable,
    ignore_case: bool,
}

impl Emitter<'_> {
    fn node(&mut self, node: &Node) -> Result<(), Reason> {
        match node {
            Node::Match {
                matcher,
                ignore_case,
            } if *ignore_case != self.ignore_case => {
                self.out
                    .push_str(if *ignore_case { "(?i:" } else { "(?-i:" });
                self.matcher(matcher)?;
                self.out.push(')');
            }
            Node::Match { matcher, .. } => self.matcher(matcher)?,
            Node::Assertion(assertion) => self.out.push_str(assertion_syntax(*assertion)),
            Node::Group { kind, body } => {
                self.out.push_str(group_open(*kind));
                self.node(body)?;
                self.out.push(')');
            }
            Node::Conditional { condition, yes, no } => {
                match condition {
                    Condition::Group(number) => {
                        let index = self
                            .groups
                            .index(*number)
                            .ok_or(Reason::UndefinedReference(*number))?;
                        let _ = write!(self.out, "(?({index})");
                    }
                    Condition::Expression(condition) => {
                        self.out.push_str("(?((?=");
                        self.node(condition)?;
                        self.out.push_str("))");
                    }
                }

                self.node(yes)?;
                if let Some(no) = no {
                    self.out.push('|');
                    self.node(no)?;
                }

                self.out.push(')');
            }
            Node::Repeat {
                node,
                min,
                max,
                lazy,
            } => {
                self.node(node)?;

                let _ = match (min, max) {
                    (0, None) => write!(self.out, "*"),
                    (1, None) => write!(self.out, "+"),
                    (0, Some(1)) => write!(self.out, "?"),
                    (min, None) => write!(self.out, "{{{min},}}"),
                    (min, Some(max)) if min == max => write!(self.out, "{{{min}}}"),
                    (min, Some(max)) => write!(self.out, "{{{min},{max}}}"),
                };

                if *lazy {
                    self.out.push('?');
                }
            }
            Node::Concat(nodes) => {
                for node in nodes {
                    self.node(node)?;
                }
            }
            Node::Alternation(branches) => {
                for (i, branch) in branches.iter().enumerate() {
                    if i > 0 {
                        self.out.push('|');
                    }

                    self.node(branch)?;
                }
            }
        }

        Ok(())
    }

    fn matcher(&mut self, matcher: &Matcher) -> Result<(), Reason> {
        match matcher {
            Matcher::Char(c) => push_char(&mut self.out, *c),
            Matcher::Any { newline: true } => self.out.push_str("(?s:.)"),
            Matcher::Any { newline: false } => self.out.push('.'),
            Matcher::Set(set) => push_set(&mut self.out, set, false),
            Matcher::Class(class) => push_class(&mut self.out, class),
            Matcher::Backreference(number) => {
                let index = self
                    .groups
                    .index(*number)
                    .ok_or(Reason::UndefinedBackref(*number))?;
                let _ = write!(self.out, r"\k<{index}>");
            }
        }

        Ok(())
    }
}

fn assertion_syntax(assertion: Assertion) -> &'static str {
    match assertion {
        Assertion::Start => r"\A",
        Assertion::LineStart => "(?m:^)",
        Assertion::End => r"\z",
        Assertion::EndOrFinalNewline => r"(?=\n?\z)",
        Assertion::LineEnd => "(?m:$)",
        Assertion::Boundary => r"\b",
        Assertion::NonBoundary => r"\B",
        Assertion::Continue => r"\G",
    }
}

fn group_open(kind: GroupKind) -> &'static str {
    match kind {
        GroupKind::Capture(_) => "(",
        GroupKind::NonCapture => "(?:",
        GroupKind::Lookahead => "(?=",
        GroupKind::NegativeLookahead => "(?!",
        GroupKind::Lookbehind => "(?<=",
        GroupKind::NegativeLookbehind => "(?<!",
        GroupKind::Atomic => "(?>",
    }
}

fn push_class(out: &mut String, class: &Class) {
    if class.subtraction.is_some() {
        out.push('[');
    }

    out.push('[');
    if class.negated {
        out.push('^');
    }

    for item in &class.items {
        match item {
            ClassItem::Char(c) => push_char(out, *c),
            ClassItem::Range(start, end) => {
                push_char(out, *start);
                out.push('-');
                push_char(out, *end);
            }
            ClassItem::Set(set) => push_set(out, set, true),
        }
    }

    out.push(']');
    if let Some(subtraction) = &class.subtraction {
        out.push_str("--");
        push_class(out, subtraction);
        out.push(']');
    }
}

fn push_set(out: &mut String, set: &Set, in_class: bool) {
    let letter = |c: char| {
        if set.negated {
            c.to_ascii_uppercase()
        } else {
            c
        }
    };
    let _ = match &set.kind {
        SetKind::Digit => write!(out, r"\{}", letter('d')),
        SetKind::Whitespace => write!(out, r"\{}", letter('s')),
        SetKind::Property(name) => write!(out, r"\{}{{{name}}}", letter('p')),
        SetKind::Word if in_class && !set.negated => write!(out, "{WORD_MEMBERS}"),
        SetKind::Word if set.negated => write!(out, "[^{WORD_MEMBERS}]"),
        SetKind::Word => write!(out, "[{WORD_MEMBERS}]"),
    };
}

fn push_char(out: &mut String, c: char) {
    if "\\.+*?()|[]{}^$#&-~ ".contains(c) {
        out.push('\\');
        out.push(c);
    } else if c.is_control() {
        let _ = write!(out, r"\x{{{:X}}}", u32::from(c));
    } else {
        out.push(c);
    }
}

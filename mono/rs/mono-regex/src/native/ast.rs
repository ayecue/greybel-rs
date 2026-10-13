#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Match {
        matcher: Matcher,
        ignore_case: bool,
    },
    Assertion(Assertion),
    Group {
        kind: GroupKind,
        body: Box<Node>,
    },
    Conditional {
        condition: Condition,
        yes: Box<Node>,
        no: Option<Box<Node>>,
    },
    Repeat {
        node: Box<Node>,
        min: u32,
        max: Option<u32>,
        lazy: bool,
    },
    Concat(Vec<Node>),
    Alternation(Vec<Node>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    Char(char),
    Any { newline: bool },
    Set(Set),
    Class(Class),
    Backreference(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub kind: SetKind,
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetKind {
    Digit,
    Whitespace,
    Word,
    Property(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub negated: bool,
    pub items: Vec<ClassItem>,
    pub subtraction: Option<Box<Class>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassItem {
    Char(char),
    Range(char, char),
    Set(Set),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assertion {
    Start,
    LineStart,
    End,
    EndOrFinalNewline,
    LineEnd,
    Boundary,
    NonBoundary,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Capture(u32),
    NonCapture,
    Lookahead,
    NegativeLookahead,
    Lookbehind,
    NegativeLookbehind,
    Atomic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Group(u32),
    Expression(Box<Node>),
}

impl Node {
    pub fn concat(mut nodes: Vec<Node>) -> Self {
        if nodes.len() == 1 {
            nodes.remove(0)
        } else {
            Self::Concat(nodes)
        }
    }

    pub fn alternation(mut branches: Vec<Node>) -> Self {
        if branches.len() == 1 {
            branches.remove(0)
        } else {
            Self::Alternation(branches)
        }
    }

    pub fn captures(&self) -> Vec<u32> {
        let mut numbers = Vec::new();

        self.collect_captures(&mut numbers);

        numbers
    }

    fn collect_captures(&self, numbers: &mut Vec<u32>) {
        match self {
            Self::Group { kind, body } => {
                if let GroupKind::Capture(number) = kind {
                    numbers.push(*number);
                }

                body.collect_captures(numbers);
            }
            Self::Conditional { condition, yes, no } => {
                if let Condition::Expression(condition) = condition {
                    condition.collect_captures(numbers);
                }

                yes.collect_captures(numbers);
                if let Some(no) = no {
                    no.collect_captures(numbers);
                }
            }
            Self::Repeat { node, .. } => node.collect_captures(numbers),
            Self::Concat(nodes) | Self::Alternation(nodes) => {
                for node in nodes {
                    node.collect_captures(numbers);
                }
            }
            Self::Match { .. } | Self::Assertion(_) => {}
        }
    }
}

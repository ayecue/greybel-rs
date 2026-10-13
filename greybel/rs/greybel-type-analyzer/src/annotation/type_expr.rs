use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    Name {
        name: String,
        arguments: Vec<TypeExpr>,
    },
    Union(Vec<TypeExpr>),
}

const MAX_DEPTH: usize = 32;

impl TypeExpr {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parser = Parser {
            text: text.as_bytes(),
            at: 0,
        };
        let expr = parser.parse_union(0)?;

        parser.skip_whitespace();

        (parser.at == parser.text.len()).then_some(expr)
    }
}

impl TypeExpr {
    pub(crate) fn parse_braced_prefix(text: &str) -> (Result<Option<Self>, ()>, &str) {
        let Some(inner) = text.strip_prefix('{') else {
            return (Ok(None), text);
        };
        let Some((ty, remainder)) = inner.split_once('}') else {
            return (Err(()), text);
        };

        (Self::parse(ty).map(Some).ok_or(()), remainder.trim())
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name { name, arguments } if arguments.is_empty() => f.write_str(name),
            Self::Name { name, arguments } => {
                let arguments: Vec<String> = arguments.iter().map(ToString::to_string).collect();

                write!(f, "{name}<{}>", arguments.join(","))
            }
            Self::Union(variants) => {
                let variants: Vec<String> = variants.iter().map(ToString::to_string).collect();

                f.write_str(&variants.join("|"))
            }
        }
    }
}

struct Parser<'a> {
    text: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn parse_union(&mut self, depth: usize) -> Option<TypeExpr> {
        if depth > MAX_DEPTH {
            return None;
        }

        let mut variants = vec![self.parse_primary(depth)?];

        while self.consume(b'|') {
            variants.push(self.parse_primary(depth)?);
        }

        Some(if variants.len() == 1 {
            variants.remove(0)
        } else {
            TypeExpr::Union(variants)
        })
    }

    fn parse_primary(&mut self, depth: usize) -> Option<TypeExpr> {
        self.skip_whitespace();

        let start = self.at;

        while self
            .text
            .get(self.at)
            .is_some_and(|&byte| Self::is_name_byte(byte))
        {
            self.at += 1;
        }

        if self.at == start {
            return None;
        }

        let name = String::from_utf8(self.text[start..self.at].to_vec()).ok()?;
        let mut arguments = Vec::new();

        if self.consume(b'<') {
            arguments.push(self.parse_union(depth + 1)?);
            while self.consume(b',') {
                arguments.push(self.parse_union(depth + 1)?);
            }

            if !self.consume(b'>') {
                return None;
            }
        }

        Some(TypeExpr::Name { name, arguments })
    }

    fn consume(&mut self, byte: u8) -> bool {
        self.skip_whitespace();

        if self.text.get(self.at) == Some(&byte) {
            self.at += 1;

            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while self.text.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'-')
    }
}

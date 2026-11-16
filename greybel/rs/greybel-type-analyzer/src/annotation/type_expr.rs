use std::fmt;

use greybel_symbols::types::FUNCTION;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    Name {
        name: String,
        arguments: Vec<TypeExpr>,
    },
    Union(Vec<TypeExpr>),
    /// `"tcp"` or `22`: the one value as a type.
    Literal(TypeLiteral),
    /// `[string, number]`: a list of the element types.
    Tuple(Vec<TypeExpr>),
    /// `value is Dog` as a return type: the function returns whether its parameter `value` is a `Dog`.
    Guard {
        parameter: String,
        ty: Box<TypeExpr>,
    },
    /// `(name: number, string) => string` or `function(name: number, string): string`; without a return type the
    /// function returns `any`.
    Function {
        parameters: Vec<FunctionTypeParameter>,
        returns: Option<Box<TypeExpr>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLiteral {
    String(String),
    /// The text of the number, as written.
    Number(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionTypeParameter {
    pub name: Option<String>,
    pub ty: TypeExpr,
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
            Self::Literal(TypeLiteral::String(text)) => {
                write!(f, "\"{}\"", text.replace('"', "\"\""))
            }
            Self::Literal(TypeLiteral::Number(text)) => f.write_str(text),
            Self::Tuple(elements) => {
                let elements: Vec<String> = elements.iter().map(ToString::to_string).collect();

                write!(f, "[{}]", elements.join(", "))
            }
            Self::Guard { parameter, ty } => write!(f, "{parameter} is {ty}"),
            Self::Function {
                parameters,
                returns,
            } => {
                let parameters: Vec<String> = parameters
                    .iter()
                    .map(|parameter| match &parameter.name {
                        Some(name) => format!("{name}: {}", parameter.ty),
                        None => parameter.ty.to_string(),
                    })
                    .collect();

                write!(f, "function({})", parameters.join(", "))?;

                match returns {
                    Some(returns) => write!(f, ": {returns}"),
                    None => Ok(()),
                }
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

        match self.text.get(self.at) {
            Some(b'"') => {
                return self
                    .parse_string()
                    .map(|text| TypeExpr::Literal(TypeLiteral::String(text)));
            }
            Some(&byte) if byte.is_ascii_digit() || byte == b'-' => {
                return self
                    .parse_number()
                    .map(|text| TypeExpr::Literal(TypeLiteral::Number(text)));
            }
            Some(b'[') => {
                self.at += 1;

                let mut elements = vec![self.parse_union(depth + 1)?];

                while self.consume(b',') {
                    elements.push(self.parse_union(depth + 1)?);
                }

                return self.consume(b']').then_some(TypeExpr::Tuple(elements));
            }
            _ => {}
        }

        if self.text.get(self.at) == Some(&b'(') {
            let parameters = self.parse_parameters(depth)?;

            if !self.consume_text(b"=>") {
                return None;
            }

            let returns = self.parse_union(depth + 1)?;

            return Some(TypeExpr::Function {
                parameters,
                returns: Some(Box::new(returns)),
            });
        }

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

        if name == FUNCTION && self.peek(b'(') {
            let parameters = self.parse_parameters(depth)?;
            let returns = if self.consume(b':') {
                Some(Box::new(self.parse_union(depth + 1)?))
            } else {
                None
            };

            return Some(TypeExpr::Function {
                parameters,
                returns,
            });
        }

        let save = self.at;

        if self.consume_word(b"is") {
            let ty = self.parse_union(depth + 1)?;

            return Some(TypeExpr::Guard {
                parameter: name,
                ty: Box::new(ty),
            });
        }

        self.at = save;

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

    /// `(name: type, type, ...)`.
    fn parse_parameters(&mut self, depth: usize) -> Option<Vec<FunctionTypeParameter>> {
        if !self.consume(b'(') {
            return None;
        }

        let mut parameters = Vec::new();

        if self.consume(b')') {
            return Some(parameters);
        }

        loop {
            let name = self.parameter_name();
            let ty = self.parse_union(depth + 1)?;

            parameters.push(FunctionTypeParameter { name, ty });

            if self.consume(b')') {
                return Some(parameters);
            }

            if !self.consume(b',') {
                return None;
            }
        }
    }

    /// A `name:` before a parameter type, consumed when there is one.
    fn parameter_name(&mut self) -> Option<String> {
        self.skip_whitespace();

        let start = self.at;

        while self
            .text
            .get(self.at)
            .is_some_and(|&byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            self.at += 1;
        }

        let end = self.at;

        if end > start && self.consume(b':') {
            return String::from_utf8(self.text[start..end].to_vec()).ok();
        }

        self.at = start;

        None
    }

    /// `"text"` with `""` for a quote.
    fn parse_string(&mut self) -> Option<String> {
        self.at += 1;

        let mut text = Vec::new();

        loop {
            match *self.text.get(self.at)? {
                b'"' if self.text.get(self.at + 1) == Some(&b'"') => {
                    text.push(b'"');
                    self.at += 2;
                }
                b'"' => {
                    self.at += 1;

                    return String::from_utf8(text).ok();
                }
                byte => {
                    text.push(byte);
                    self.at += 1;
                }
            }
        }
    }

    fn parse_number(&mut self) -> Option<String> {
        let start = self.at;

        if self.text.get(self.at) == Some(&b'-') {
            self.at += 1;
        }

        let digits = self.at;

        while self
            .text
            .get(self.at)
            .is_some_and(|&byte| byte.is_ascii_digit() || byte == b'.')
        {
            self.at += 1;
        }

        let text = std::str::from_utf8(&self.text[start..self.at]).ok()?;

        (self.at > digits && text.parse::<f64>().is_ok()).then(|| text.to_owned())
    }

    /// `word` after whitespace, not followed by a name character.
    fn consume_word(&mut self, word: &[u8]) -> bool {
        let save = self.at;

        self.skip_whitespace();

        let ends = self
            .text
            .get(self.at + word.len())
            .is_none_or(|&byte| !Self::is_name_byte(byte));

        if self.at > save && self.text[self.at..].starts_with(word) && ends {
            self.at += word.len();

            return true;
        }

        self.at = save;

        false
    }

    fn peek(&mut self, byte: u8) -> bool {
        self.skip_whitespace();

        self.text.get(self.at) == Some(&byte)
    }

    fn consume_text(&mut self, text: &[u8]) -> bool {
        self.skip_whitespace();

        if self.text[self.at..].starts_with(text) {
            self.at += text.len();

            true
        } else {
            false
        }
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

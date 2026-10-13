use super::{BOOL, BYTES, CHAR, FLATTEN, LIST, MATCH, Parser, RECORD, STREAM, STRING};
use crate::error::DefinitionError;
use crate::model::{
    DocumentationRef, Field, FieldId, Length, Literal, Member, MemberId, NumberKind, Payload,
    Range, Span, StringRequirementContainer, TypeRef, Variant, VariantId,
};
use crate::syntax::lexer::{Token, TokenKind};

impl Parser {
    pub(super) fn fields(&mut self) -> Result<Vec<Field>, DefinitionError> {
        self.field_list(|_, _, _, span| {
            Err(DefinitionError::new(
                span,
                "only a message's last field is a stream",
            ))
        })
    }

    pub(super) fn field_list(
        &mut self,
        mut stream: impl FnMut(&mut Self, DocumentationRef, String, Span) -> Result<(), DefinitionError>,
    ) -> Result<Vec<Field>, DefinitionError> {
        self.expect('{')?;

        let mut fields = Vec::new();
        let mut streamed = false;

        while !self.consume('}') {
            let docs = self.docs();
            let tags = self.tags()?;
            let span = self.peek().span;

            if streamed {
                return Err(DefinitionError::new(
                    span,
                    "a stream is a message's last field",
                ));
            }

            let flatten = self.is_flatten();

            if flatten {
                self.bump();
            }

            let name = self.name()?;
            let optional = self.consume('?');

            self.expect(':')?;
            if self.is_keyword(STREAM) {
                if optional || flatten {
                    return Err(DefinitionError::new(
                        span,
                        "a stream is neither optional nor flattened",
                    ));
                }

                if !tags.is_empty() {
                    return Err(DefinitionError::new(span, "a stream takes no tags"));
                }

                stream(self, docs, name, span)?;
                streamed = true;
            } else {
                let ty = self.type_ref()?;
                let default = if self.consume('=') {
                    Some(self.literal()?)
                } else {
                    None
                };

                fields.push(Field {
                    id: FieldId(Self::take_id(&mut self.fields)),
                    docs,
                    tags,
                    name,
                    optional,
                    flatten,
                    ty,
                    default,
                    span,
                });
            }

            if !self.consume(',') {
                self.expect('}')?;
                break;
            }
        }

        Ok(fields)
    }

    pub(super) fn take_id(counter: &mut u32) -> u32 {
        let id = *counter;

        *counter += 1;

        id
    }

    pub(super) fn is_flatten(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(keyword) if keyword == FLATTEN)
            && matches!(
                self.tokens.get(self.position + 1),
                Some(Token {
                    kind: TokenKind::Ident(_),
                    ..
                })
            )
    }

    pub(super) fn variants(&mut self) -> Result<Vec<Variant>, DefinitionError> {
        self.expect('{')?;

        let mut variants = Vec::new();

        while !self.consume('}') {
            let docs = self.docs();
            let tags = self.tags()?;
            let span = self.peek().span;
            let name = self.name()?;
            let payload = if self.consume('(') {
                let value = self.type_ref()?;

                self.expect(')')?;

                Payload::Value(value)
            } else if self.is('{') {
                Payload::Fields(self.fields()?)
            } else {
                Payload::Unit
            };

            variants.push(Variant {
                id: VariantId(Self::take_id(&mut self.variants)),
                docs,
                tags,
                name,
                payload,
                span,
            });
            if !self.consume(',') {
                self.expect('}')?;
                break;
            }
        }

        Ok(variants)
    }

    pub(super) fn members(&mut self) -> Result<Vec<Member>, DefinitionError> {
        self.expect('{')?;

        let mut members = Vec::new();

        while !self.consume('}') {
            let docs = self.docs();
            let tags = self.tags()?;
            let span = self.peek().span;
            let name = self.name()?;
            let value = if self.consume('=') {
                Some(self.number()?)
            } else {
                None
            };

            members.push(Member {
                id: MemberId(Self::take_id(&mut self.members)),
                docs,
                tags,
                name,
                value,
                span,
            });
            if !self.consume(',') {
                self.expect('}')?;
                break;
            }
        }

        Ok(members)
    }

    pub(super) fn type_ref(&mut self) -> Result<TypeRef, DefinitionError> {
        let name = self.name()?;
        let ty = match name.as_str() {
            BOOL => TypeRef::Bool,
            CHAR => TypeRef::Char,
            STRING => TypeRef::String(self.string_requirements()?),
            BYTES => TypeRef::Bytes(self.length_requirement(Self::size_bound)?),
            LIST => {
                let item = self.argument()?;

                TypeRef::List(Box::new(item), self.length_requirement(Self::count_bound)?)
            }
            RECORD => {
                let value = self.argument()?;

                TypeRef::Record(Box::new(value), self.length_requirement(Self::count_bound)?)
            }
            _ => match NumberKind::from_keyword(&name) {
                Some(kind) => TypeRef::Number(kind, self.bounds()?),
                None => TypeRef::Declared(name),
            },
        };

        Ok(if self.consume('?') {
            TypeRef::Nullable(Box::new(ty))
        } else {
            ty
        })
    }

    pub(super) fn length_requirement(
        &mut self,
        bound: fn(&mut Self) -> Result<u32, DefinitionError>,
    ) -> Result<Length, DefinitionError> {
        if !self.consume('(') {
            return Ok(Length::ANY);
        }

        let length = self.length(bound)?;

        self.expect(')')?;

        Ok(length)
    }

    fn string_requirements(&mut self) -> Result<StringRequirementContainer, DefinitionError> {
        let mut requirements = StringRequirementContainer::default();

        if !self.consume('(') {
            return Ok(requirements);
        }

        let (mut length, mut pattern) = (false, false);

        loop {
            let span = self.peek().span;

            if self.consume_keyword(MATCH) {
                if std::mem::replace(&mut pattern, true) {
                    return Err(DefinitionError::new(span, "a string matches one pattern"));
                }

                requirements.pattern = Some(self.text()?);
            } else {
                if std::mem::replace(&mut length, true) {
                    return Err(DefinitionError::new(span, "a string has one length"));
                }

                requirements.length = self.length(Self::size_bound)?;
            }

            if !self.consume(',') {
                break;
            }
        }

        self.expect(')')?;

        Ok(requirements)
    }

    fn length(
        &mut self,
        bound: fn(&mut Self) -> Result<u32, DefinitionError>,
    ) -> Result<Length, DefinitionError> {
        let (min, max) = self.range(bound)?;

        Ok(Length {
            min: min.unwrap_or(0),
            max,
        })
    }

    fn bounds(&mut self) -> Result<Range, DefinitionError> {
        if !self.consume('(') {
            return Ok(Range::default());
        }

        let (min, max) = self.range(|parser| match parser.number_literal()? {
            Literal::Number(digits) => Ok(digits),
            _ => unreachable!("a number literal is a number"),
        })?;

        self.expect(')')?;

        Ok(Range { min, max })
    }

    fn range<T>(
        &mut self,
        bound: impl Fn(&mut Self) -> Result<T, DefinitionError>,
    ) -> Result<(Option<T>, Option<T>), DefinitionError> {
        let span = self.peek().span;
        let min = if self.is('.') {
            None
        } else {
            Some(bound(self)?)
        };

        self.expect('.')?;
        self.expect('.')?;
        if self.consume('=') {
            return Ok((min, Some(bound(self)?)));
        }

        if min.is_none() || !(self.is(')') || self.is(',')) {
            return Err(DefinitionError::new(
                span,
                "a range is `least..=most`, `..=most` or `least..`",
            ));
        }

        Ok((min, None))
    }

    pub(super) fn argument(&mut self) -> Result<TypeRef, DefinitionError> {
        self.expect('<')?;

        let ty = self.type_ref()?;

        self.expect('>')?;

        Ok(ty)
    }
}

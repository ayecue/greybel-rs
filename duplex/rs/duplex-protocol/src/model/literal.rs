use std::num::{ParseFloatError, ParseIntError};

use super::{
    Definition, DocumentationRef, Field, Length, NumberKind, Range, Span, TypeKind, TypeRef,
};
use crate::pattern::{PatternError, PatternTable};

#[derive(Debug, thiserror::Error)]
pub enum LiteralError {
    #[error("names the constant `{name}`, of another type than {ty}")]
    ConstantType { name: String, ty: String },
    #[error("isn't one character")]
    Character,
    #[error("isn't a value of {ty}")]
    Type { ty: String },
    #[error("isn't a number")]
    Float(#[source] ParseFloatError),
    #[error("is out of the range of f32")]
    FloatRange,
    #[error("isn't an integer")]
    Integer(#[source] ParseIntError),
    #[error("is out of the range of {}", kind.keyword())]
    Range { kind: NumberKind },
    #[error("names no constant, and `{enumeration}` is no enumeration")]
    Enumeration { enumeration: String, member: String },
    #[error("names no member of `{enumeration}`")]
    Member { enumeration: String, member: String },
    #[error("`{digits}` {source}")]
    Bound {
        digits: String,
        #[source]
        source: Box<LiteralError>,
    },
    #[error("has a least above its most")]
    BoundOrder,
    #[error("is {length} bytes long, out of {requirement}")]
    Length { length: usize, requirement: Length },
    #[error(transparent)]
    Pattern(#[from] PatternError),
    #[error("doesn't match `{pattern}`")]
    PatternMatch { pattern: String },
    #[error("is out of its bounds")]
    Bounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Number(String),
    Text(String),
    Null,
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Integer(i128),
    Float(f64),
    Text(String),
    Char(char),
    Null,
    Member { name: String, value: u32 },
}

impl Value {
    fn compare(left: &Self, right: &Self) -> std::cmp::Ordering {
        match (left, right) {
            (Self::Integer(left), Self::Integer(right)) => left.cmp(right),
            (Self::Float(left), Self::Float(right)) => left.total_cmp(right),
            _ => std::cmp::Ordering::Equal,
        }
    }

    pub(crate) fn canonical(&self) -> String {
        match self {
            Self::Bool(value) => format!("b{}", u8::from(*value)),
            Self::Integer(value) => format!("i{value}"),
            Self::Float(value) => format!("f{value:?}"),
            Self::Text(value) => format!("t{value}"),
            Self::Char(value) => format!("c{value}"),
            Self::Null => "n".to_owned(),
            Self::Member { value, .. } => format!("m{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constant {
    pub docs: DocumentationRef,
    pub name: String,
    pub ty: TypeRef,
    pub value: Literal,
    pub(crate) span: Span,
}

impl Definition {
    /// Resolves `literal` as `ty`, including constant references and requirement validation.
    ///
    /// # Errors
    ///
    /// Returns an error if conversion to `ty` fails or requirements are violated.
    pub fn value_of(&self, literal: &Literal, ty: &TypeRef) -> Result<Value, LiteralError> {
        if let Literal::Name(name) = literal
            && let Some(constant) = self.constant(name)
        {
            if !constant.ty.is_same_type(ty) && !constant.ty.is_same_type(ty.non_null()) {
                return Err(LiteralError::ConstantType {
                    name: name.clone(),
                    ty: ty.keyword(),
                });
            }

            return self.value_of(&constant.value, &constant.ty);
        }

        let value = match (ty, literal) {
            (TypeRef::Nullable(_), Literal::Null) => return Ok(Value::Null),
            (TypeRef::Nullable(inner), _) => return self.value_of(literal, inner),
            (TypeRef::Bool, Literal::Bool(value)) => Ok(Value::Bool(*value)),
            (TypeRef::String(_), Literal::Text(text)) => Ok(Value::Text(text.clone())),
            (TypeRef::Char, Literal::Text(text)) => {
                let mut chars = text.chars();

                match (chars.next(), chars.next()) {
                    (Some(char), None) => Ok(Value::Char(char)),
                    _ => Err(LiteralError::Character),
                }
            }
            (TypeRef::Number(kind, _), Literal::Number(digits)) => Self::number(*kind, digits),
            (TypeRef::Declared(name), Literal::Name(member)) => self.member(name, member),
            _ => Err(LiteralError::Type { ty: ty.keyword() }),
        }?;

        Self::require(ty, &value)?;

        Ok(value)
    }

    /// Resolves numeric `bounds` to values of `kind`.
    ///
    /// # Errors
    ///
    /// Returns an error for a bound invalid for `kind` or a minimum greater than the maximum.
    pub fn bounds_of(
        kind: NumberKind,
        bounds: &Range,
    ) -> Result<(Option<Value>, Option<Value>), LiteralError> {
        let resolve = |digits: &Option<String>| {
            digits
                .as_deref()
                .map(|digits| {
                    Self::number(kind, digits).map_err(|source| LiteralError::Bound {
                        digits: digits.to_owned(),
                        source: Box::new(source),
                    })
                })
                .transpose()
        };
        let (min, max) = (resolve(&bounds.min)?, resolve(&bounds.max)?);

        if let (Some(min), Some(max)) = (&min, &max)
            && Value::compare(min, max).is_gt()
        {
            return Err(LiteralError::BoundOrder);
        }

        Ok((min, max))
    }

    fn require(ty: &TypeRef, value: &Value) -> Result<(), LiteralError> {
        match (ty, value) {
            (TypeRef::String(requirements), Value::Text(text)) => {
                let length = requirements.length;

                if !length.contains(text.len() as u64) {
                    return Err(LiteralError::Length {
                        length: text.len(),
                        requirement: length,
                    });
                }

                if let Some(pattern) = &requirements.pattern
                    && !PatternTable::compile(pattern)?.matches(text.as_bytes())
                {
                    return Err(LiteralError::PatternMatch {
                        pattern: pattern.clone(),
                    });
                }

                Ok(())
            }
            (TypeRef::Number(kind, bounds), value) => {
                let (min, max) = Self::bounds_of(*kind, bounds)?;

                if min.is_some_and(|min| Value::compare(value, &min).is_lt())
                    || max.is_some_and(|max| Value::compare(value, &max).is_gt())
                {
                    return Err(LiteralError::Bounds);
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    #[must_use]
    pub fn default_of(&self, field: &Field) -> Option<Value> {
        self.value_of(field.default.as_ref()?, &field.ty).ok()
    }

    #[must_use]
    pub fn constant(&self, name: &str) -> Option<&Constant> {
        self.constants.iter().find(|constant| constant.name == name)
    }

    #[must_use]
    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    fn number(kind: NumberKind, digits: &str) -> Result<Value, LiteralError> {
        if kind.is_float() {
            let value: f64 = digits.parse().map_err(LiteralError::Float)?;

            #[expect(
                clippy::cast_possible_truncation,
                reason = "rounding to f32 tells whether the value is within its range"
            )]
            let infinite_as_f32 = (value as f32).is_infinite();

            if kind == NumberKind::F32 && infinite_as_f32 {
                return Err(LiteralError::FloatRange);
            }

            return Ok(Value::Float(value));
        }

        let value: i128 = digits.parse().map_err(LiteralError::Integer)?;
        let (minimum, maximum) = kind.range();

        if value < minimum || value > maximum {
            return Err(LiteralError::Range { kind });
        }

        Ok(Value::Integer(value))
    }

    fn member(&self, enumeration: &str, member: &str) -> Result<Value, LiteralError> {
        let Some(TypeKind::Enumeration(members)) = self
            .declaration(enumeration)
            .map(|declaration| &declaration.kind)
        else {
            return Err(LiteralError::Enumeration {
                enumeration: enumeration.to_owned(),
                member: member.to_owned(),
            });
        };

        members
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name == member)
            .map(|(index, hit)| Value::Member {
                name: hit.name.clone(),
                value: hit
                    .value
                    .unwrap_or(u32::try_from(index).expect("members are fewer than 2^32")),
            })
            .ok_or_else(|| LiteralError::Member {
                enumeration: enumeration.to_owned(),
                member: member.to_owned(),
            })
    }
}

impl NumberKind {
    fn range(self) -> (i128, i128) {
        match self {
            Self::U8 => (0, u8::MAX.into()),
            Self::U16 => (0, u16::MAX.into()),
            Self::U32 | Self::Fixed32 => (0, u32::MAX.into()),
            Self::U64 | Self::Fixed64 => (0, u64::MAX.into()),
            Self::I8 => (i8::MIN.into(), i8::MAX.into()),
            Self::I16 => (i16::MIN.into(), i16::MAX.into()),
            Self::I32 => (i32::MIN.into(), i32::MAX.into()),
            Self::I64 => (i64::MIN.into(), i64::MAX.into()),
            Self::F32 | Self::F64 => (i128::MIN, i128::MAX),
        }
    }
}

impl TypeRef {
    #[must_use]
    pub fn keyword(&self) -> String {
        match self {
            Self::Bool => "bool".to_owned(),
            Self::Char => "char".to_owned(),
            Self::String(_) => "string".to_owned(),
            Self::Bytes(_) => "bytes".to_owned(),
            Self::Number(kind, _) => kind.keyword().to_owned(),
            Self::List(item, _) => format!("list<{}>", item.keyword()),
            Self::Record(value, _) => format!("record<{}>", value.keyword()),
            Self::Nullable(inner) => format!("{}?", inner.keyword()),
            Self::Declared(name) => name.clone(),
        }
    }
}

use std::borrow::Cow;
use std::collections::BTreeMap;

use super::{BinaryDecode, BinaryEncode, BinaryReader, Bytes, Fixed32, Fixed64};
use crate::definition::{Requirement, Violation};
use crate::error::Error;

const BYTES: usize = 256;

/// DFA transitions indexed by `base[state] + class`; `owner` validates each transition.
#[doc(hidden)]
#[derive(Debug)]
pub struct Pattern {
    pub classes: [u8; BYTES],
    pub start: u16,
    pub base: &'static [u32],
    pub next: &'static [u16],
    pub owner: &'static [u16],
    pub accepting: &'static [bool],
}

impl Pattern {
    #[must_use]
    pub fn matches(&self, bytes: &[u8]) -> bool {
        let mut state = self.start;

        for &byte in bytes {
            let slot = self.base[usize::from(state)] as usize
                + usize::from(self.classes[usize::from(byte)]);

            if self.owner.get(slot) != Some(&state) {
                return false;
            }

            state = self.next[slot];
        }

        self.accepting[usize::from(state)]
    }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub enum Validation {
    Any,
    Length {
        min: u32,
        max: u32,
        pattern: Option<&'static Pattern>,
    },
    Integer {
        min: i128,
        max: i128,
    },
    Float {
        min: f64,
        max: f64,
    },
    Items {
        min: u32,
        max: u32,
        item: &'static Validation,
    },
}

impl Validation {
    pub(crate) fn is_within(&self, length: usize) -> Result<(), Error> {
        let (Self::Length { min, max, .. } | Self::Items { min, max, .. }) = *self else {
            return Ok(());
        };

        if length < min as usize {
            return Err(Error::invalid(Requirement::MinLength));
        }

        if length > max as usize {
            return Err(Error::invalid(Requirement::MaxLength));
        }

        Ok(())
    }

    pub(crate) fn is_less_than(&self, length: usize) -> Result<(), Error> {
        match *self {
            Self::Length { max, .. } | Self::Items { max, .. } if length > max as usize => {
                Err(Error::invalid(Requirement::MaxLength))
            }
            _ => Ok(()),
        }
    }

    pub(super) fn matches_pattern(&self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::Length {
                pattern: Some(pattern),
                ..
            } if !pattern.matches(bytes) => Err(Error::invalid(Requirement::Pattern)),
            _ => Ok(()),
        }
    }

    fn is_text(&self, text: &str) -> Result<(), Error> {
        self.is_within(text.len())?;

        self.matches_pattern(text.as_bytes())
    }

    fn is_int(&self, value: i128) -> Result<(), Error> {
        match *self {
            Self::Integer { min, .. } if value < min => Err(Error::invalid(Requirement::Minimum)),
            Self::Integer { max, .. } if value > max => Err(Error::invalid(Requirement::Maximum)),
            _ => Ok(()),
        }
    }

    fn is_float(&self, value: f64) -> Result<(), Error> {
        match *self {
            Self::Float { min, .. } if value.is_nan() || value < min => {
                Err(Error::invalid(Requirement::Minimum))
            }
            Self::Float { max, .. } if value > max => Err(Error::invalid(Requirement::Maximum)),
            _ => Ok(()),
        }
    }

    pub(super) fn item(&self) -> &Self {
        match self {
            Self::Items { item, .. } => item,
            _ => &Self::Any,
        }
    }
}

impl Error {
    fn invalid(requirement: Requirement) -> Self {
        Self::Invalid(Violation {
            path: Vec::new(),
            requirement,
        })
    }

    #[doc(hidden)]
    #[must_use]
    pub fn invalid_field(mut self, position: usize) -> Self {
        if let Self::Invalid(violation) = &mut self {
            violation
                .path
                .insert(0, u32::try_from(position).unwrap_or(u32::MAX));
        }

        self
    }
}

#[doc(hidden)]
pub trait Validatable {
    fn validate(&self, validation: &Validation) -> Result<(), Error>;
}

#[doc(hidden)]
pub trait ValidatedEncode: BinaryEncode + Validatable {}

impl<T: BinaryEncode + Validatable + ?Sized> ValidatedEncode for T {}

#[doc(hidden)]
pub trait ValidatedDecode: BinaryDecode {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error>;
}

macro_rules! integers {
    ($($ty:ty),*) => {
        $(
            impl Validatable for $ty {
                fn validate(&self, validation: &Validation) -> Result<(), Error> {
                    validation.is_int(i128::from(*self))
                }
            }

            impl ValidatedDecode for $ty {
                fn read_with_validation(reader: &mut BinaryReader<'_>, validation: &Validation) -> Result<Self, Error> {
                    let value = Self::read(reader)?;
                    if reader.validates() {
                        value.validate(validation)?;
                    }
                    Ok(value)
                }
            }
        )*
    };
}

integers!(u8, u16, u32, u64, i8, i16, i32, i64);

macro_rules! wrapped {
    ($($ty:ty => $inner:ty, $method:ident),*) => {
        $(
            impl Validatable for $ty {
                fn validate(&self, validation: &Validation) -> Result<(), Error> {
                    validation.$method(<$inner>::from(self.0))
                }
            }

            impl ValidatedDecode for $ty {
                fn read_with_validation(reader: &mut BinaryReader<'_>, validation: &Validation) -> Result<Self, Error> {
                    let value = Self::read(reader)?;
                    if reader.validates() {
                        value.validate(validation)?;
                    }
                    Ok(value)
                }
            }
        )*
    };
}

wrapped!(Fixed32 => i128, is_int, Fixed64 => i128, is_int);

macro_rules! floats {
    ($($ty:ty),*) => {
        $(
            impl Validatable for $ty {
                fn validate(&self, validation: &Validation) -> Result<(), Error> {
                    validation.is_float(f64::from(*self))
                }
            }

            impl ValidatedDecode for $ty {
                fn read_with_validation(reader: &mut BinaryReader<'_>, validation: &Validation) -> Result<Self, Error> {
                    let value = Self::read(reader)?;
                    if reader.validates() {
                        value.validate(validation)?;
                    }
                    Ok(value)
                }
            }
        )*
    };
}

floats!(f32, f64);

macro_rules! unvalidated {
    ($($ty:ty),*) => {
        $(
            impl Validatable for $ty {
                fn validate(&self, _: &Validation) -> Result<(), Error> {
                    Ok(())
                }
            }

            impl ValidatedDecode for $ty {
                fn read_with_validation(reader: &mut BinaryReader<'_>, _: &Validation) -> Result<Self, Error> {
                    Self::read(reader)
                }
            }
        )*
    };
}

unvalidated!(bool, char, (), std::convert::Infallible);

impl Validatable for str {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        validation.is_text(self)
    }
}

impl Validatable for String {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        validation.is_text(self)
    }
}

impl ValidatedDecode for String {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read(reader);
        }

        let length = reader.length()?;

        validation.is_within(length)?;

        let bytes = reader.take(length)?;

        validation.matches_pattern(bytes)?;

        BinaryReader::utf8(bytes).map(str::to_owned)
    }
}

impl Validatable for Bytes {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        validation.is_within(self.0.len())
    }
}

impl ValidatedDecode for Bytes {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read(reader);
        }

        let length = reader.length()?;

        validation.is_within(length)?;

        Ok(Self(reader.take(length)?.to_vec()))
    }
}

impl<T: Validatable> Validatable for Option<T> {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        self.as_ref()
            .map_or(Ok(()), |value| value.validate(validation))
    }
}

impl<T: ValidatedDecode> ValidatedDecode for Option<T> {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if reader.boolean()? {
            T::read_with_validation(reader, validation).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<T: Validatable> Validatable for [T] {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        validation.is_within(self.len())?;

        let item = validation.item();

        self.iter().enumerate().try_for_each(|(index, value)| {
            value
                .validate(item)
                .map_err(|error| error.invalid_field(index))
        })
    }
}

impl<T: Validatable> Validatable for Vec<T> {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        self.as_slice().validate(validation)
    }
}

impl<T: ValidatedDecode> ValidatedDecode for Vec<T> {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read(reader);
        }

        let (length, capacity) = reader.length_and_capacity()?;

        validation.is_within(length)?;

        let item = validation.item();
        let mut items = Vec::with_capacity(capacity);

        for index in 0..length {
            items.push(
                T::read_with_validation(reader, item)
                    .map_err(|error| error.invalid_field(index))?,
            );
        }

        Ok(items)
    }
}

impl<V: Validatable> Validatable for BTreeMap<String, V> {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        validation.is_within(self.len())?;

        let item = validation.item();

        self.values().enumerate().try_for_each(|(index, value)| {
            value
                .validate(item)
                .map_err(|error| error.invalid_field(index))
        })
    }
}

impl<V: ValidatedDecode> ValidatedDecode for BTreeMap<String, V> {
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read(reader);
        }

        let (length, _) = reader.length_and_capacity()?;

        validation.is_within(length)?;

        let item = validation.item();
        let mut entries = BTreeMap::new();

        for index in 0..length {
            let key = reader.string()?;
            let value = V::read_with_validation(reader, item)
                .map_err(|error| error.invalid_field(index))?;

            entries.insert(key, value);
        }

        Ok(entries)
    }
}

impl<B: Validatable + ToOwned + ?Sized> Validatable for Cow<'_, B> {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        self.as_ref().validate(validation)
    }
}

impl<B: ToOwned + ?Sized> ValidatedDecode for Cow<'_, B>
where
    B::Owned: ValidatedDecode,
{
    fn read_with_validation(
        reader: &mut BinaryReader<'_>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        B::Owned::read_with_validation(reader, validation).map(Cow::Owned)
    }
}

impl<T: Validatable + ?Sized> Validatable for &T {
    fn validate(&self, validation: &Validation) -> Result<(), Error> {
        (**self).validate(validation)
    }
}

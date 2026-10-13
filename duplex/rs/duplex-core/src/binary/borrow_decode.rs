use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{BinaryDecode, BinaryReader, Fixed32, Fixed64, ValidatedDecode, Validation};
use crate::error::Error;

#[doc(hidden)]
pub trait BorrowDecode<'de>: Sized {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error>;

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error>;
}

macro_rules! owned {
    ($($ty:ty),*) => {
        $(
            impl<'de> BorrowDecode<'de> for $ty {
                fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
                    <Self as BinaryDecode>::read(reader)
                }

                fn read_ref_with_validation(
                    reader: &mut BinaryReader<'de>,
                    validation: &Validation,
                ) -> Result<Self, Error> {
                    <Self as ValidatedDecode>::read_with_validation(reader, validation)
                }
            }
        )*
    };
}

owned!(
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    i8,
    i16,
    i32,
    i64,
    f32,
    f64,
    Fixed32,
    Fixed64,
    (),
    Infallible
);

impl<'de> BorrowDecode<'de> for &'de str {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
        reader.str()
    }

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return reader.str();
        }

        let length = reader.length()?;

        validation.is_within(length)?;

        let bytes = reader.take(length)?;

        validation.matches_pattern(bytes)?;

        BinaryReader::utf8(bytes)
    }
}

impl<'de> BorrowDecode<'de> for &'de [u8] {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
        reader.bytes()
    }

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return reader.bytes();
        }

        let length = reader.length()?;

        validation.is_within(length)?;

        reader.take(length)
    }
}

impl<'de, T: BorrowDecode<'de>> BorrowDecode<'de> for Option<T> {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
        if reader.boolean()? {
            T::read_ref(reader).map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if reader.boolean()? {
            T::read_ref_with_validation(reader, validation).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de, T: BorrowDecode<'de>> BorrowDecode<'de> for Vec<T> {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
        let (length, capacity) = reader.length_and_capacity()?;
        let mut items = Vec::with_capacity(capacity);

        for _ in 0..length {
            items.push(T::read_ref(reader)?);
        }

        Ok(items)
    }

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read_ref(reader);
        }

        let (length, capacity) = reader.length_and_capacity()?;

        validation.is_within(length)?;

        let item = validation.item();
        let mut items = Vec::with_capacity(capacity);

        for index in 0..length {
            items.push(
                T::read_ref_with_validation(reader, item)
                    .map_err(|error| error.invalid_field(index))?,
            );
        }

        Ok(items)
    }
}

impl<'de, V: BorrowDecode<'de>> BorrowDecode<'de> for BTreeMap<&'de str, V> {
    fn read_ref(reader: &mut BinaryReader<'de>) -> Result<Self, Error> {
        let (length, _) = reader.length_and_capacity()?;
        let mut entries = BTreeMap::new();

        for _ in 0..length {
            let key = reader.str()?;

            entries.insert(key, V::read_ref(reader)?);
        }

        Ok(entries)
    }

    fn read_ref_with_validation(
        reader: &mut BinaryReader<'de>,
        validation: &Validation,
    ) -> Result<Self, Error> {
        if !reader.validates() {
            return Self::read_ref(reader);
        }

        let (length, _) = reader.length_and_capacity()?;

        validation.is_within(length)?;

        let item = validation.item();
        let mut entries = BTreeMap::new();

        for index in 0..length {
            let key = reader.str()?;
            let value = V::read_ref_with_validation(reader, item)
                .map_err(|error| error.invalid_field(index))?;

            entries.insert(key, value);
        }

        Ok(entries)
    }
}

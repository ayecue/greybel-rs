use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter};
use crate::error::Error;

const ABSENT: u8 = 0;
const PRESENT: u8 = 1;

impl<T: BinaryEncode> BinaryEncode for Option<T> {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        match self {
            None => {
                writer.byte(ABSENT);

                Ok(())
            }
            Some(value) => {
                writer.byte(PRESENT);

                value.write(writer)
            }
        }
    }
}

impl<T: BinaryDecode> BinaryDecode for Option<T> {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        if reader.boolean()? {
            T::read(reader).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<T: BinaryEncode> BinaryEncode for [T] {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.length(self.len())?;

        self.iter().try_for_each(|item| item.write(writer))
    }
}

impl<T: BinaryEncode> BinaryEncode for Vec<T> {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        self.as_slice().write(writer)
    }
}

impl<T: BinaryDecode> BinaryDecode for Vec<T> {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        let (length, capacity) = reader.length_and_capacity()?;
        let mut items = Vec::with_capacity(capacity);

        for _ in 0..length {
            items.push(T::read(reader)?);
        }

        Ok(items)
    }
}

impl<V: BinaryEncode> BinaryEncode for BTreeMap<String, V> {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.length(self.len())?;

        self.iter().try_for_each(|(key, value)| {
            writer.string(key)?;

            value.write(writer)
        })
    }
}

impl<V: BinaryDecode> BinaryDecode for BTreeMap<String, V> {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        let (length, _) = reader.length_and_capacity()?;
        let mut entries = BTreeMap::new();

        for _ in 0..length {
            let key = reader.string()?;

            entries.insert(key, V::read(reader)?);
        }

        Ok(entries)
    }
}

impl<B: BinaryEncode + ToOwned + ?Sized> BinaryEncode for Cow<'_, B> {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        self.as_ref().write(writer)
    }
}

impl<B: ToOwned + ?Sized> BinaryDecode for Cow<'_, B>
where
    B::Owned: BinaryDecode,
{
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        B::Owned::read(reader).map(Cow::Owned)
    }
}

impl<T: BinaryEncode + ?Sized> BinaryEncode for &T {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        (**self).write(writer)
    }
}

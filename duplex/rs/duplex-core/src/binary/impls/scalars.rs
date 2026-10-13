use std::convert::Infallible;

use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter};
use crate::error::{DecodeFailure, Error};

macro_rules! scalar {
    ($($ty:ty => $method:ident),* $(,)?) => {
        $(
            impl BinaryEncode for $ty {
                fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
                    writer.$method(*self);
                    Ok(())
                }
            }

            impl BinaryDecode for $ty {
                fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
                    reader.$method()
                }
            }
        )*
    };
}

scalar! {
    bool => boolean,
    char => char,
    u8 => u8,
    u16 => u16,
    u32 => u32,
    u64 => u64,
    i8 => i8,
    i16 => i16,
    i32 => i32,
    i64 => i64,
    f32 => f32,
    f64 => f64,
}

impl BinaryEncode for str {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.string(self)
    }
}

impl BinaryEncode for String {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.string(self)
    }
}

impl BinaryDecode for String {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        reader.string()
    }
}

const ERR_NO_FAILURE: DecodeFailure = DecodeFailure::UndeclaredFailure;

impl BinaryEncode for Infallible {
    fn write(&self, _writer: &mut BinaryWriter) -> Result<(), Error> {
        match *self {}
    }
}

impl BinaryDecode for Infallible {
    fn read(_reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        Err(Error::Decode(ERR_NO_FAILURE))
    }
}

impl BinaryEncode for () {
    fn write(&self, _writer: &mut BinaryWriter) -> Result<(), Error> {
        Ok(())
    }
}

impl BinaryDecode for () {
    fn read(_reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        Ok(())
    }
}

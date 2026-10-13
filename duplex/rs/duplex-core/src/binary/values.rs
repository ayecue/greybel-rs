use std::ops::Deref;

use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter};
use crate::error::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Four-byte little-endian `u32`.
pub struct Fixed32(pub u32);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Eight-byte little-endian `u64`.
pub struct Fixed64(pub u64);

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Byte vector with a length prefix.
pub struct Bytes(pub Vec<u8>);

impl From<u32> for Fixed32 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Fixed32> for u32 {
    fn from(value: Fixed32) -> Self {
        value.0
    }
}

impl From<u64> for Fixed64 {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Fixed64> for u64 {
    fn from(value: Fixed64) -> Self {
        value.0
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for Bytes {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl BinaryEncode for Fixed32 {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.fixed32(self.0);

        Ok(())
    }
}

impl BinaryDecode for Fixed32 {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        reader.fixed32().map(Self)
    }
}

impl BinaryEncode for Fixed64 {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.fixed64(self.0);

        Ok(())
    }
}

impl BinaryDecode for Fixed64 {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        reader.fixed64().map(Self)
    }
}

impl BinaryEncode for Bytes {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.bytes(&self.0)
    }
}

impl BinaryDecode for Bytes {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        reader.bytes().map(Self::from)
    }
}

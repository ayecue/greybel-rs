use super::{BinaryReader, BinaryWriter};
use crate::error::Error;

#[doc(hidden)]
pub trait BinaryEncode {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error>;

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut writer = BinaryWriter::new();

        self.write(&mut writer)?;

        Ok(writer.into_bytes())
    }
}

#[doc(hidden)]
pub trait BinaryDecode: Sized {
    fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error>;

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = BinaryReader::new(bytes);
        let value = Self::read(&mut reader)?;

        reader.finish()?;

        Ok(value)
    }
}

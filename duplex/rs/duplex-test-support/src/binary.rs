//! Duplex binary encoding and hexadecimal test-data helpers.

use std::fmt::Write;

use duplex_core::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, Error};

/// Encodes `value` with the latest protocol minor.
pub fn bytes<T: BinaryEncode + ?Sized>(value: &T) -> Vec<u8> {
    encode(value, BinaryWriter::new())
}

/// Encodes `value` with protocol minor `minor`.
pub fn bytes_at<T: BinaryEncode + ?Sized>(value: &T, minor: u16) -> Vec<u8> {
    let mut writer = BinaryWriter::new();

    writer.set_minor(minor);

    encode(value, writer)
}

fn encode<T: BinaryEncode + ?Sized>(value: &T, mut writer: BinaryWriter) -> Vec<u8> {
    value.write(&mut writer).expect("the test value is encode");

    writer.into_bytes()
}

/// Decodes one `T` from `bytes`, rejecting trailing bytes.
pub fn decode<T: BinaryDecode>(bytes: &[u8]) -> Result<T, Error> {
    let mut reader = BinaryReader::new(bytes);
    let value = T::read(&mut reader)?;

    reader.finish()?;

    Ok(value)
}

/// Returns `bytes` as lowercase hexadecimal.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");

        hex
    })
}

/// Decodes hexadecimal `hex` to bytes and panics on invalid input.
#[must_use]
pub fn unhex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hexadecimal"))
        .collect()
}

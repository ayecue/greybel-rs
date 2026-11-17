use super::varint::{CONTINUATION, Varuint, ZigZag};
use crate::error::{EncodeFailure, Error};

const ERR_TOO_LONG: EncodeFailure = EncodeFailure::LengthOverflow;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct BinaryWriter {
    bytes: Vec<u8>,
    minor: u16,
    length_bytes: u8,
}

impl Default for BinaryWriter {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl BinaryWriter {
    pub const NEWEST: u16 = u16::MAX;

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
            minor: Self::NEWEST,
            length_bytes: 1,
        }
    }

    #[must_use]
    pub fn minor(&self) -> u16 {
        self.minor
    }

    pub fn set_minor(&mut self, minor: u16) {
        self.minor = minor;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub(crate) fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            minor: Self::NEWEST,
            length_bytes: 1,
        }
    }

    pub(crate) fn bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }

    pub(crate) fn pad(&mut self, count: usize) {
        self.bytes.resize(self.bytes.len() + count, 0);
    }

    pub fn truncate(&mut self, length: usize) {
        self.bytes.truncate(length);
    }

    pub(crate) fn remove_front(&mut self, length: usize) {
        self.bytes.drain(..length);
    }

    pub fn patch(&mut self, offset: usize, bytes: &[u8]) {
        self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    /// Reserves a length prefix for [`Self::finish_reserved_length`].
    pub(crate) fn reserve_length(&mut self) -> usize {
        let offset = self.bytes.len();

        match self.length_bytes {
            1 => self.bytes.push(0),
            bytes => self.bytes.resize(offset + usize::from(bytes), 0),
        }

        offset
    }

    /// Writes a reserved length prefix and shifts the body when its encoded size changes.
    pub(crate) fn finish_reserved_length(&mut self, offset: usize) -> Result<(), Error> {
        let prefix_len = usize::from(self.length_bytes);
        let length = Self::valid_length(self.bytes.len() - offset - prefix_len)?;

        if prefix_len == 1 && length < u32::from(CONTINUATION) {
            self.bytes[offset] = length.to_le_bytes()[0];

            return Ok(());
        }

        let prefix = Varuint::new(u64::from(length));
        let prefix = prefix.as_bytes();
        let size = prefix.len();

        if size != prefix_len {
            let start = offset + prefix_len;

            if size > prefix_len {
                self.bytes
                    .splice(start..start, std::iter::repeat_n(0, size - prefix_len));
            } else {
                self.bytes.drain(offset + size..start);
            }

            self.length_bytes = u8::try_from(size).expect("a varuint takes at most 10 bytes");
        }

        self.bytes[offset..offset + size].copy_from_slice(prefix);

        Ok(())
    }

    pub fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub fn bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        self.length(value.len())?;
        self.raw(value);

        Ok(())
    }

    pub fn boolean(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    pub fn varuint(&mut self, value: u64) {
        if value < u64::from(CONTINUATION) {
            self.bytes.push(value.to_le_bytes()[0]);
        } else {
            self.bytes.extend_from_slice(Varuint::new(value).as_bytes());
        }
    }

    pub fn varint(&mut self, value: i64) {
        self.varuint(value.zigzag());
    }

    pub fn length(&mut self, length: usize) -> Result<(), Error> {
        let length = Self::valid_length(length)?;

        self.varuint(u64::from(length));

        Ok(())
    }

    pub fn string(&mut self, value: &str) -> Result<(), Error> {
        self.length(value.len())?;
        self.raw(value.as_bytes());

        Ok(())
    }

    pub fn char(&mut self, value: char) {
        self.varuint(u64::from(u32::from(value)));
    }

    pub fn u8(&mut self, value: u8) {
        self.byte(value);
    }

    pub fn i8(&mut self, value: i8) {
        self.raw(&value.to_le_bytes());
    }

    pub fn u16(&mut self, value: u16) {
        self.varuint(u64::from(value));
    }

    pub fn u32(&mut self, value: u32) {
        self.varuint(u64::from(value));
    }

    pub fn u64(&mut self, value: u64) {
        self.varuint(value);
    }

    pub fn i16(&mut self, value: i16) {
        self.varint(i64::from(value));
    }

    pub fn i32(&mut self, value: i32) {
        self.varint(i64::from(value));
    }

    pub fn i64(&mut self, value: i64) {
        self.varint(value);
    }

    pub fn f32(&mut self, value: f32) {
        self.raw(&value.to_le_bytes());
    }

    pub fn f64(&mut self, value: f64) {
        self.raw(&value.to_le_bytes());
    }

    pub fn fixed32(&mut self, value: u32) {
        self.raw(&value.to_le_bytes());
    }

    pub fn fixed64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }

    fn valid_length(length: usize) -> Result<u32, Error> {
        u32::try_from(length).map_err(|_| Error::Encode(ERR_TOO_LONG))
    }
}

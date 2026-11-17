use super::BinaryWriter;
use super::varint::{CONTINUATION, PAYLOAD, SHIFT, UnZigZag};
use crate::error::{DecodeFailure, Error};

const ERR_UNEXPECTED_END: DecodeFailure = DecodeFailure::InputEnd;
const ERR_TRAILING_INPUT: DecodeFailure = DecodeFailure::InputRemainder;
const ERR_TOO_LARGE: DecodeFailure = DecodeFailure::IntegerOverflow;
const ERR_INVALID_BOOLEAN: DecodeFailure = DecodeFailure::InvalidBoolean;
const ERR_INVALID_CHAR: DecodeFailure = DecodeFailure::InvalidCharacter;
const ERR_INVALID_FLAGS: DecodeFailure = DecodeFailure::InvalidFlags;
const ERR_INVALID_UTF8: DecodeFailure = DecodeFailure::InvalidUtf8;
const U64_BITS: u32 = u64::BITS;

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct BinaryReader<'a> {
    bytes: &'a [u8],
    position: usize,
    minor: u16,
    validating: bool,
}

impl<'a> BinaryReader<'a> {
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            minor: BinaryWriter::NEWEST,
            validating: false,
        }
    }

    #[must_use]
    pub fn with_validation(mut self, validating: bool) -> Self {
        self.validating = validating;

        self
    }

    #[must_use]
    pub fn validates(&self) -> bool {
        self.validating
    }

    #[must_use]
    pub fn with_minor(mut self, minor: u16) -> Self {
        self.minor = minor;

        self
    }

    #[must_use]
    pub fn minor(&self) -> u16 {
        self.minor
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }

    #[must_use]
    pub fn unread_len(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub fn finish(&self) -> Result<(), Error> {
        if self.unread_len() == 0 {
            Ok(())
        } else {
            Err(Self::error(ERR_TRAILING_INPUT))
        }
    }

    pub fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        if length > self.unread_len() {
            return Err(Self::error(ERR_UNEXPECTED_END));
        }

        let start = self.position;

        self.position += length;

        Ok(&self.bytes[start..self.position])
    }

    pub fn take_all(&mut self) -> &'a [u8] {
        let remainder = &self.bytes[self.position..];

        self.position = self.bytes.len();

        remainder
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let bytes = self.take(N)?;
        let mut array = [0; N];

        array.copy_from_slice(bytes);

        Ok(array)
    }

    pub fn byte(&mut self) -> Result<u8, Error> {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| Self::error(ERR_UNEXPECTED_END))?;

        self.position += 1;

        Ok(byte)
    }

    pub fn flags(&mut self, mask: u8) -> Result<u8, Error> {
        let flags = self.byte()?;

        if flags & !mask == 0 {
            Ok(flags)
        } else {
            Err(Self::error(ERR_INVALID_FLAGS))
        }
    }

    pub fn boolean(&mut self) -> Result<bool, Error> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Self::error(ERR_INVALID_BOOLEAN)),
        }
    }

    #[inline]
    pub fn varuint(&mut self) -> Result<u64, Error> {
        if let Some(&byte) = self.bytes.get(self.position)
            && byte & CONTINUATION == 0
        {
            self.position += 1;

            return Ok(u64::from(byte));
        }

        if let Some(&[first, second]) = self.bytes.get(self.position..self.position + 2)
            && second & CONTINUATION == 0
        {
            self.position += 2;

            return Ok(u64::from(first & PAYLOAD) | (u64::from(second) << SHIFT));
        }

        self.long_varuint()
    }

    #[cold]
    #[inline(never)]
    fn long_varuint(&mut self) -> Result<u64, Error> {
        let mut value = 0_u64;
        let mut shift = 0;

        loop {
            let byte = self.byte()?;
            let part = u64::from(byte & PAYLOAD);

            if shift >= U64_BITS || (part << shift) >> shift != part {
                return Err(Self::error(ERR_TOO_LARGE));
            }

            value |= part << shift;
            if byte & CONTINUATION == 0 {
                return Ok(value);
            }

            shift += SHIFT;
        }
    }

    #[inline]
    pub fn varint(&mut self) -> Result<i64, Error> {
        Ok(self.varuint()?.unzigzag())
    }

    #[inline]
    pub fn length(&mut self) -> Result<usize, Error> {
        let length = u32::try_from(self.varuint()?).map_err(|_| Self::error(ERR_TOO_LARGE))?;

        usize::try_from(length).map_err(|_| Self::error(ERR_TOO_LARGE))
    }

    pub fn length_and_capacity(&mut self) -> Result<(usize, usize), Error> {
        let length = self.length()?;

        Ok((length, length.min(self.unread_len())))
    }

    pub fn str(&mut self) -> Result<&'a str, Error> {
        let length = self.length()?;
        let bytes = self.take(length)?;

        Self::utf8(bytes)
    }

    pub(crate) fn utf8(bytes: &[u8]) -> Result<&str, Error> {
        str::from_utf8(bytes).map_err(|_| Self::error(ERR_INVALID_UTF8))
    }

    pub fn string(&mut self) -> Result<String, Error> {
        self.str().map(str::to_owned)
    }

    pub fn char(&mut self) -> Result<char, Error> {
        let code = u32::try_from(self.varuint()?).map_err(|_| Self::error(ERR_INVALID_CHAR))?;

        char::from_u32(code).ok_or_else(|| Self::error(ERR_INVALID_CHAR))
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        self.byte()
    }

    pub fn i8(&mut self) -> Result<i8, Error> {
        Ok(i8::from_le_bytes(self.array()?))
    }

    pub fn u16(&mut self) -> Result<u16, Error> {
        u16::try_from(self.varuint()?).map_err(|_| Self::error(ERR_TOO_LARGE))
    }

    #[inline]
    pub fn u32(&mut self) -> Result<u32, Error> {
        u32::try_from(self.varuint()?).map_err(|_| Self::error(ERR_TOO_LARGE))
    }

    #[inline]
    pub fn u64(&mut self) -> Result<u64, Error> {
        self.varuint()
    }

    pub fn i16(&mut self) -> Result<i16, Error> {
        i16::try_from(self.varint()?).map_err(|_| Self::error(ERR_TOO_LARGE))
    }

    pub fn i32(&mut self) -> Result<i32, Error> {
        i32::try_from(self.varint()?).map_err(|_| Self::error(ERR_TOO_LARGE))
    }

    #[inline]
    pub fn i64(&mut self) -> Result<i64, Error> {
        self.varint()
    }

    pub fn f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    pub fn f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    pub fn fixed32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn fixed64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub fn bytes(&mut self) -> Result<&'a [u8], Error> {
        let length = self.length()?;

        self.take(length)
    }

    fn error(failure: DecodeFailure) -> Error {
        Error::Decode(failure)
    }
}

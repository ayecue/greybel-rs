use std::ops::Range;

use super::SliceKey;
use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter};
use crate::definition::{BatchHeader, CORE, FrameKind};
use crate::error::{DecodeFailure, Error};

pub(crate) const CORE_MAJOR: u8 = CORE.major.to_le_bytes()[0];
const _: () = assert!(
    CORE.major <= u8::MAX as u16,
    "a core's major fits in a byte"
);

pub(crate) const LAST_FRAME: u8 = 0x80;
const FRAGMENT_HEAD: usize = 2;
const ERR_NO_SLICES: DecodeFailure = DecodeFailure::EmptyBatch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Slice {
    pub(crate) key: SliceKey,
    pub(crate) frames: Range<usize>,
}

pub(crate) struct BatchReader<'a> {
    reader: BinaryReader<'a>,
    core: u8,
    remaining_slices: u32,
}

impl<'a> BatchReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut reader = BinaryReader::new(bytes);
        let header = BatchHeader::read(&mut reader)?;

        if header.slices == 0 {
            return Err(Error::Decode(ERR_NO_SLICES));
        }

        Ok(Self {
            reader,
            core: header.core,
            remaining_slices: header.slices,
        })
    }

    pub(crate) fn is_of_this_core(&self) -> bool {
        self.core == CORE_MAJOR
    }

    pub(crate) fn core(&self) -> u8 {
        self.core
    }

    fn slice(&mut self) -> Result<Slice, Error> {
        self.remaining_slices -= 1;

        let key = SliceKey::read(&mut self.reader)?;
        let length = if self.remaining_slices == 0 {
            self.reader.unread_len()
        } else {
            self.reader.length()?
        };
        let start = self.reader.position();

        self.reader.take(length)?;

        Ok(Slice {
            key,
            frames: start..start + length,
        })
    }
}

impl Iterator for BatchReader<'_> {
    type Item = Result<Slice, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_slices == 0 {
            return None;
        }

        let slice = self.slice();

        if slice.is_err() {
            self.remaining_slices = 0;
        }

        Some(slice)
    }
}

pub(crate) struct BatchWriter<'b> {
    batch: &'b mut BinaryWriter,
    remaining_slices: u32,
}

impl<'b> BatchWriter<'b> {
    pub(crate) fn new(batch: &'b mut BinaryWriter, core: u8, slices: u32) -> Result<Self, Error> {
        Self::with_reserved_prefix(batch, 0, core, slices)
    }

    /// Starts a batch after `reserved_prefix` driver-framing bytes.
    pub(crate) fn with_reserved_prefix(
        batch: &'b mut BinaryWriter,
        reserved_prefix: usize,
        core: u8,
        slices: u32,
    ) -> Result<Self, Error> {
        batch.clear();
        batch.pad(reserved_prefix);
        BatchHeader { core, slices }.write(batch)?;

        Ok(Self {
            batch,
            remaining_slices: slices,
        })
    }

    /// Writes a single-slice batch key; the caller then writes frames.
    pub(crate) fn key(self, key: SliceKey) -> Result<(), Error> {
        key.write(self.batch)
    }

    /// Returns the byte width of the length prefix at the start of `bytes`.
    pub(crate) fn length_bytes(bytes: &[u8]) -> Result<usize, Error> {
        let mut length = BinaryReader::new(bytes);

        length.length()?;

        Ok(length.position())
    }

    pub(crate) fn slice(&mut self, key: SliceKey, frames: &[u8], last: usize) -> Result<(), Error> {
        self.remaining_slices -= 1;
        key.write(self.batch)?;

        let length_bytes = Self::length_bytes(&frames[last + 1..])?;

        if self.remaining_slices > 0 {
            self.batch.length(frames.len() - length_bytes)?;
        }

        self.batch.raw(&frames[..last]);
        self.batch.byte(frames[last] | LAST_FRAME);
        self.batch.raw(&frames[last + 1 + length_bytes..]);

        Ok(())
    }

    pub(crate) fn fragment(
        &mut self,
        key: SliceKey,
        bytes: &[u8],
        ends: bool,
    ) -> Result<(), Error> {
        self.remaining_slices -= 1;
        key.write(self.batch)?;
        if self.remaining_slices > 0 {
            self.batch.length(FRAGMENT_HEAD + bytes.len())?;
        }

        self.batch.byte(FrameKind::Fragment.byte() | LAST_FRAME);
        self.batch.byte(u8::from(ends));
        self.batch.raw(bytes);

        Ok(())
    }

    pub(crate) fn last_slice(&mut self, key: SliceKey, frames: &[u8]) -> Result<(), Error> {
        self.remaining_slices -= 1;
        key.write(self.batch)?;
        self.batch.raw(frames);

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/batch.rs"]
mod tests;

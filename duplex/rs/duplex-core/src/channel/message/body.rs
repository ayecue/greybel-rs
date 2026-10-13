use std::ops::Range;
use std::rc::Rc;

use crate::binary::{BinaryReader, BorrowDecode, ValidatedDecode, Validation};
use crate::definition::{Requirement, Violation};
use crate::error::Error;

#[derive(Debug, Clone)]
pub(crate) struct Body {
    batch: Rc<[u8]>,
    start: u32,
    end: u32,
    minor: u16,
    validating: bool,
    validation: &'static Validation,
}

impl Body {
    pub(crate) fn from_chunk(bytes: Vec<u8>) -> Self {
        let length = bytes.len();

        Self::new(bytes.into(), 0..length, crate::binary::BinaryWriter::NEWEST)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.batch[self.start as usize..self.end as usize]
    }

    pub(crate) fn minor(&self) -> u16 {
        self.minor
    }

    pub(crate) fn new(batch: Rc<[u8]>, range: Range<usize>, minor: u16) -> Self {
        Self {
            batch,
            start: u32::try_from(range.start).unwrap_or(u32::MAX),
            end: u32::try_from(range.end).unwrap_or(u32::MAX),
            minor,
            validating: false,
            validation: &Validation::Any,
        }
    }

    pub(crate) fn validate(
        &mut self,
        validation: &'static Validation,
        limit: u32,
    ) -> Result<(), Error> {
        if self.end - self.start > limit {
            return Err(Error::Invalid(Violation {
                path: Vec::new(),
                requirement: Requirement::Limit,
            }));
        }

        self.validating = true;
        self.validation = validation;

        Ok(())
    }

    /// Decodes a body-borrowing value; `decode` returns an owned value.
    pub(crate) fn decode_ref<'a, M: BorrowDecode<'a>>(&'a self) -> Result<M, Error> {
        let mut reader = BinaryReader::new(self.bytes())
            .with_minor(self.minor)
            .with_validation(self.validating);
        let value = M::read_ref_with_validation(&mut reader, self.validation)?;

        reader.finish()?;

        Ok(value)
    }

    pub(crate) fn decode<M: ValidatedDecode>(&self) -> Result<M, Error> {
        let mut reader = BinaryReader::new(self.bytes())
            .with_minor(self.minor)
            .with_validation(self.validating);
        let value = M::read_with_validation(&mut reader, self.validation)?;

        reader.finish()?;

        Ok(value)
    }
}

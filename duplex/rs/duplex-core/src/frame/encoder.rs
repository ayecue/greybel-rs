use super::{Control, Rejection};
use crate::binary::{BinaryEncode, BinaryWriter};
use crate::definition::{FrameKind, Requirement, Violation};
use crate::error::Error;
use crate::protocol::MessageSegment;

pub(crate) const DECLARED_FAILURE: u32 = 0;
const KEY_SHIFT: u32 = 1;
const HAS_ID: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MessageKey {
    pub(crate) type_id: u32,
    pub(crate) has_id: bool,
}

impl MessageKey {
    pub(crate) fn of(key: u32) -> Self {
        Self {
            type_id: key >> KEY_SHIFT,
            has_id: key & HAS_ID == HAS_ID,
        }
    }

    pub(crate) fn to_u32(self) -> u32 {
        (self.type_id << KEY_SHIFT) | (u32::from(self.has_id) * HAS_ID)
    }
}

pub(crate) struct FrameEncoder<'w> {
    writer: &'w mut BinaryWriter,
    minor: u16,
    last: Option<usize>,
    limit: usize,
}

impl<'w> FrameEncoder<'w> {
    pub(crate) fn new(writer: &'w mut BinaryWriter, minor: u16) -> Self {
        Self {
            writer,
            minor,
            last: None,
            limit: usize::MAX,
        }
    }

    pub(crate) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    fn within(limit: usize, writer: &BinaryWriter, start: usize) -> Result<(), Error> {
        if writer.len() - start > limit {
            return Err(Error::Invalid(Violation {
                path: Vec::new(),
                requirement: Requirement::Limit,
            }));
        }

        Ok(())
    }

    pub(crate) fn last(&self) -> Option<usize> {
        self.last
    }

    pub(crate) fn raw(&mut self, kind: u8, payload: &[u8]) -> Result<(), Error> {
        let start = self.writer.len();

        self.writer.byte(kind);
        self.writer.length(payload.len())?;
        self.writer.raw(payload);
        self.last = Some(start);

        Ok(())
    }

    pub(crate) fn message(
        &mut self,
        type_id: u32,
        id: Option<u32>,
        write: impl FnOnce(&mut BinaryWriter) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let limit = self.limit;

        self.frame(FrameKind::Message, |writer| {
            let key = MessageKey {
                type_id,
                has_id: id.is_some(),
            };

            writer.u32(key.to_u32());
            if let Some(id) = id {
                writer.u32(id);
            }

            let start = writer.len();

            write(writer)?;

            Self::within(limit, writer, start)
        })
    }

    pub(crate) fn segment<V: BinaryEncode + ?Sized>(
        &mut self,
        segment: MessageSegment,
        id: u32,
        value: &V,
    ) -> Result<(), Error> {
        assert!(
            segment != MessageSegment::Message,
            "a message is written by `message`"
        );

        let kind = segment.frame_kind();
        let limit = self.limit;

        self.frame(kind, |writer| {
            writer.u32(id);
            if segment == MessageSegment::Failure {
                writer.u32(DECLARED_FAILURE);
            }

            let start = writer.len();

            value.write(writer)?;

            Self::within(limit, writer, start)
        })
    }

    pub(crate) fn credit(&mut self, kind: FrameKind, id: u32, count: u32) -> Result<(), Error> {
        self.frame(kind, |writer| {
            writer.u32(id);
            writer.u32(count);

            Ok(())
        })
    }

    pub(crate) fn rejection(&mut self, id: u32, rejection: &Rejection) -> Result<(), Error> {
        self.frame(FrameKind::Failure, |writer| {
            writer.u32(id);
            writer.u32(u32::from(rejection.code()));

            match rejection {
                Rejection::Invalid(violation) => violation.write(writer),
                Rejection::Code(_) => Ok(()),
            }
        })
    }

    pub(crate) fn cancel(&mut self, id: u32) -> Result<(), Error> {
        self.frame(FrameKind::Cancel, |writer| {
            writer.u32(id);

            Ok(())
        })
    }

    pub(crate) fn control(&mut self, control: &Control) -> Result<(), Error> {
        self.frame(control.kind(), |writer| control.write(writer))
    }

    fn frame(
        &mut self,
        kind: FrameKind,
        write: impl FnOnce(&mut BinaryWriter) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let start = self.writer.len();

        self.writer.set_minor(self.minor);
        self.writer.byte(kind.byte());

        let length = self.writer.reserve_length();
        let outcome = write(self.writer).and_then(|()| self.writer.finish_reserved_length(length));

        if outcome.is_err() {
            self.writer.truncate(start);
        } else {
            self.last = Some(start);
        }

        outcome
    }
}

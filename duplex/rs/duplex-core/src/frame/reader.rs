use std::ops::Range;

use super::batch::LAST_FRAME;
use super::encoder::{DECLARED_FAILURE, MessageKey};
use super::{Control, Rejection};
use crate::binary::{BinaryDecode, BinaryReader, CONTINUATION};
use crate::definition::{ErrorCode, FrameKind, Violation};
use crate::error::{DecodeFailure, Error};
use crate::protocol::MessageSegment;

const KIND: u8 = !LAST_FRAME;

pub(crate) struct RawFrame {
    pub(crate) kind: u8,
    pub(crate) payload: Range<usize>,
}

pub(crate) struct FrameReader<'a> {
    batch: &'a [u8],
    position: usize,
    end: usize,
    last_length: usize,
}

impl<'a> FrameReader<'a> {
    pub(crate) fn new(batch: &'a [u8], frames: Range<usize>) -> Self {
        Self {
            batch,
            position: frames.start,
            end: frames.end,
            last_length: 0,
        }
    }

    pub(crate) fn raw(&mut self) -> Result<Option<RawFrame>, Error> {
        if self.position >= self.end {
            return Ok(None);
        }

        let kind = self.batch[self.position];

        if kind & LAST_FRAME == 0
            && let Some(&length) = self.batch.get(self.position + 1)
            && length & CONTINUATION == 0
        {
            let start = self.position + 2;
            let end = start + usize::from(length);

            if end <= self.end {
                self.last_length = end - self.position;
                self.position = end;

                return Ok(Some(RawFrame {
                    kind: kind & KIND,
                    payload: start..end,
                }));
            }
        }

        let mut reader = BinaryReader::new(&self.batch[self.position..self.end]);
        let kind = reader.byte()?;
        let length = if kind & LAST_FRAME == 0 {
            reader.length()?
        } else {
            reader.unread_len()
        };
        let start = self.position + reader.position();

        reader.take(length)?;
        self.last_length = start + length - self.position;
        self.position = start + length;

        Ok(Some(RawFrame {
            kind: kind & KIND,
            payload: start..start + length,
        }))
    }

    pub(crate) fn last_length(&self) -> usize {
        self.last_length
    }

    fn frame(&mut self) -> Result<Option<Frame>, Error> {
        while let Some(raw) = self.raw()? {
            if let Ok(kind) = FrameKind::try_from(u32::from(raw.kind)) {
                let payload = &self.batch[raw.payload.clone()];

                return Frame::read(kind, &mut BinaryReader::new(payload), raw.payload.start)
                    .map(Some);
            }
        }

        Ok(None)
    }
}

impl Iterator for FrameReader<'_> {
    type Item = Result<Frame, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.frame().transpose();

        if matches!(frame, Some(Err(_))) {
            self.position = self.end;
        }

        frame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameHead {
    pub(crate) kind: FrameKind,
    pub(crate) length: usize,
    pub(crate) id: Option<u32>,
}

impl Frame {
    pub(crate) fn kind_and_id(&self) -> (Option<FrameKind>, Option<u32>) {
        match self {
            Self::Message { id, .. } => (Some(FrameKind::Message), *id),
            Self::Segment { segment, id, .. } => (Some(segment.frame_kind()), Some(*id)),
            Self::Rejection { id, .. } => (Some(FrameKind::Failure), Some(*id)),
            _ => (None, None),
        }
    }
}

impl FrameHead {
    pub(crate) fn read(bytes: &[u8]) -> Option<Self> {
        let mut reader = BinaryReader::new(bytes);
        let kind = FrameKind::try_from(u32::from(reader.byte().ok()? & KIND)).ok()?;
        let length = reader.length().ok()?;
        let length = reader.position() + length;
        let id = match kind {
            FrameKind::Message => reader
                .u32()
                .ok()
                .map(MessageKey::of)
                .filter(|key| key.has_id)
                .and_then(|_| reader.u32().ok()),
            FrameKind::ResponseEnd
            | FrameKind::Failure
            | FrameKind::Cancel
            | FrameKind::ResponseStart
            | FrameKind::ResponseItem
            | FrameKind::ResponseCredit
            | FrameKind::MessageItem
            | FrameKind::MessageEnd
            | FrameKind::MessageCredit => reader.u32().ok(),
            _ => None,
        };

        Some(Self { kind, length, id })
    }
}

const _: () = assert!(size_of::<Frame>() <= 32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Frame {
    Message {
        type_id: u32,
        id: Option<u32>,
        body: Range<usize>,
    },
    Segment {
        segment: MessageSegment,
        id: u32,
        body: Range<usize>,
    },
    Rejection {
        id: u32,
        rejection: Rejection,
    },
    Credit {
        for_message: bool,
        id: u32,
        count: u32,
    },
    Cancel {
        id: u32,
    },
    Fragment {
        ends: bool,
        bytes: Range<usize>,
    },
    Control(Box<Control>),
}

impl Frame {
    fn read(kind: FrameKind, payload: &mut BinaryReader<'_>, start: usize) -> Result<Self, Error> {
        let body = |payload: &BinaryReader<'_>| {
            let offset = start + payload.position();

            offset..offset + payload.unread_len()
        };
        let segment = |segment, payload: &mut BinaryReader<'_>| -> Result<Self, Error> {
            let id = payload.u32()?;

            Ok(Self::Segment {
                segment,
                id,
                body: body(payload),
            })
        };

        Ok(match kind {
            FrameKind::Message => {
                let MessageKey { type_id, has_id } = MessageKey::of(payload.u32()?);
                let id = if has_id { Some(payload.u32()?) } else { None };

                Self::Message {
                    type_id,
                    id,
                    body: body(payload),
                }
            }
            FrameKind::ResponseEnd => segment(MessageSegment::ResponseEnd, payload)?,
            FrameKind::ResponseStart => segment(MessageSegment::ResponseStart, payload)?,
            FrameKind::ResponseItem => segment(MessageSegment::ResponseItem, payload)?,
            FrameKind::MessageItem => segment(MessageSegment::MessageItem, payload)?,
            FrameKind::MessageEnd => segment(MessageSegment::MessageEnd, payload)?,
            FrameKind::Failure => {
                let id = payload.u32()?;

                match payload.u32()? {
                    DECLARED_FAILURE => Self::Segment {
                        segment: MessageSegment::Failure,
                        id,
                        body: body(payload),
                    },
                    code => Self::Rejection {
                        id,
                        rejection: match ErrorCode::try_from(code)
                            .map_err(|code| Error::Decode(DecodeFailure::UnknownErrorCode(code)))?
                        {
                            ErrorCode::Invalid => {
                                Rejection::Invalid(Box::new(Violation::read(payload)?))
                            }
                            code => Rejection::Code(code),
                        },
                    },
                }
            }
            FrameKind::ResponseCredit | FrameKind::MessageCredit => Self::Credit {
                for_message: kind == FrameKind::MessageCredit,
                id: payload.u32()?,
                count: payload.u32()?,
            },
            FrameKind::Cancel => Self::Cancel { id: payload.u32()? },
            FrameKind::Fragment => {
                let ends = payload.boolean()?;

                Self::Fragment {
                    ends,
                    bytes: body(payload),
                }
            }
            FrameKind::Listen
            | FrameKind::Open
            | FrameKind::Ack
            | FrameKind::Refuse
            | FrameKind::Ping
            | FrameKind::Pong
            | FrameKind::Close => Self::Control(Box::new(Control::read(kind, payload)?)),
        })
    }
}

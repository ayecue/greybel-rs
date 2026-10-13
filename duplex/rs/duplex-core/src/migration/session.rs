use super::path::MigrationPath;
use crate::binary::{BinaryEncode, BinaryWriter};
use crate::channel::message::{Body, PendingBody};
use crate::definition::{ErrorCode, Role, Version};
use crate::error::Error;
use crate::frame::FrameEncoder;
use crate::protocol::MessageSegment;

#[derive(Clone)]
pub(crate) struct Session {
    pub(crate) core: Version,
    pub(crate) protocol: Version,
    pub(crate) migration_path: Option<MigrationPath>,
}

pub(crate) struct MessageWrite<'a> {
    pub(crate) type_id: u32,
    pub(crate) id: Option<u32>,
    pub(crate) since: u16,
    pub(crate) body: MessageWriteBody<'a>,
}

pub(crate) enum MessageWriteBody<'a> {
    Value(&'a dyn BinaryEncode),
    Pending(&'a PendingBody, u16),
}

pub(crate) struct SegmentWrite<'a> {
    pub(crate) segment: MessageSegment,
    pub(crate) id: u32,
    pub(crate) type_id: u32,
    pub(crate) value: &'a dyn BinaryEncode,
}

struct Raw<'b>(&'b [u8]);

impl BinaryEncode for Raw<'_> {
    fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        writer.raw(self.0);

        Ok(())
    }
}

impl Session {
    pub(crate) fn minor(&self) -> u16 {
        self.protocol.minor
    }

    #[inline]
    pub(crate) fn write_message(
        &self,
        frames: &mut FrameEncoder<'_>,
        message: &MessageWrite<'_>,
    ) -> Result<(), Error> {
        let Some(path) = &self.migration_path else {
            if message.since > self.minor() {
                return Err(Error::Unavailable);
            }

            return frames.message(message.type_id, message.id, |writer| match message.body {
                MessageWriteBody::Value(value) => value.write(writer),
                MessageWriteBody::Pending(pending_body, newest) => {
                    pending_body.write_body(writer, newest)
                }
            });
        };

        self.write_message_via_migration(path, frames, message)
    }

    #[cold]
    #[inline(never)]
    fn write_message_via_migration(
        &self,
        path: &MigrationPath,
        frames: &mut FrameEncoder<'_>,
        message: &MessageWrite<'_>,
    ) -> Result<(), Error> {
        let chunk;
        let bytes = match message.body {
            MessageWriteBody::Value(value) => {
                chunk = value.to_bytes()?;

                &chunk
            }
            MessageWriteBody::Pending(pending_body, _) => &pending_body.body,
        };
        let (id, body) = path
            .down(
                MessageSegment::Message,
                Role::Client,
                message.type_id,
                bytes,
                self.minor(),
            )
            .map_err(Error::of_migration)?;

        frames.message(id, message.id, |writer| Raw(&body).write(writer))
    }

    #[inline]
    pub(crate) fn write_segment(
        &self,
        frames: &mut FrameEncoder<'_>,
        segment: &SegmentWrite<'_>,
    ) -> Result<(), Error> {
        let Some(path) = &self.migration_path else {
            return frames.segment(segment.segment, segment.id, segment.value);
        };

        self.write_segment_via_migration(path, frames, segment)
    }

    #[cold]
    #[inline(never)]
    fn write_segment_via_migration(
        &self,
        path: &MigrationPath,
        frames: &mut FrameEncoder<'_>,
        segment: &SegmentWrite<'_>,
    ) -> Result<(), Error> {
        let (_, chunk) = path
            .down(
                segment.segment,
                Self::receiver_for_write(segment.segment),
                segment.type_id,
                &segment.value.to_bytes()?,
                self.minor(),
            )
            .map_err(Error::of_migration)?;

        frames.segment(segment.segment, segment.id, &Raw(&chunk))
    }

    #[inline]
    pub(crate) fn read_message(&self, type_id: u32, body: Body) -> Result<(u32, Body), ErrorCode> {
        let Some(path) = &self.migration_path else {
            return Ok((type_id, body));
        };

        Self::read_message_via_migration(path, type_id, &body)
    }

    #[cold]
    #[inline(never)]
    fn read_message_via_migration(
        path: &MigrationPath,
        type_id: u32,
        body: &Body,
    ) -> Result<(u32, Body), ErrorCode> {
        let (type_id, bytes) = path.up(
            MessageSegment::Message,
            Role::Provider,
            type_id,
            body.bytes(),
            body.minor(),
        )?;

        Ok((type_id, Body::from_chunk(bytes)))
    }

    #[inline]
    pub(crate) fn read_segment(
        &self,
        segment: MessageSegment,
        type_id: u32,
        body: Body,
    ) -> Result<Body, Error> {
        let Some(path) = &self.migration_path else {
            return Ok(body);
        };

        Self::read_segment_via_migration(path, segment, type_id, &body)
    }

    #[cold]
    #[inline(never)]
    fn read_segment_via_migration(
        path: &MigrationPath,
        segment: MessageSegment,
        type_id: u32,
        body: &Body,
    ) -> Result<Body, Error> {
        let receiver = Self::receiver_for_read(segment);
        let type_id = if segment.is_response() {
            path.oldest(receiver, type_id)
                .map_err(Error::of_migration)?
        } else {
            type_id
        };

        path.up(segment, receiver, type_id, body.bytes(), body.minor())
            .map(|(_, bytes)| Body::from_chunk(bytes))
            .map_err(Error::of_migration)
    }

    fn receiver_for_write(segment: MessageSegment) -> Role {
        if segment.is_response() {
            Role::Provider
        } else {
            Role::Client
        }
    }

    fn receiver_for_read(segment: MessageSegment) -> Role {
        if segment.is_response() {
            Role::Client
        } else {
            Role::Provider
        }
    }
}

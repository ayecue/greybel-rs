mod path;
mod session;

pub(crate) use path::{MigrationPath, MigrationTable};
pub(crate) use session::{MessageWrite, MessageWriteBody, SegmentWrite, Session};

use crate::binary::{BinaryReader, BinaryWriter};
use crate::definition::{ErrorCode, Role, Version};
use crate::protocol::{MessageSegment, Peer, Protocol, VersionInfo};

#[doc(hidden)]
pub trait Migration: 'static {
    type Older: Peer;
    type Newer: Peer;

    fn migrate(
        &self,
        segment: MessageSegment,
        receiver: Role,
        type_id: u32,
        reader: &mut BinaryReader<'_>,
        writer: &mut BinaryWriter,
    ) -> Result<u32, ErrorCode>;

    fn older_message(&self, receiver: Role, type_id: u32) -> Option<u32>;
}

pub(crate) trait MigrationStep {
    fn from(&self) -> Version;
    fn versions(&self) -> &'static [VersionInfo];
    fn migrate(
        &self,
        segment: MessageSegment,
        receiver: Role,
        type_id: u32,
        body: &[u8],
        read_minor: u16,
        write_minor: u16,
    ) -> Result<(u32, Vec<u8>), ErrorCode>;

    fn older_message(&self, receiver: Role, type_id: u32) -> Option<u32>;
}

impl<M: Migration> MigrationStep for M {
    fn from(&self) -> Version {
        <<M::Older as Peer>::Protocol as Protocol>::VERSION
    }

    fn versions(&self) -> &'static [VersionInfo] {
        <<M::Older as Peer>::Protocol as Protocol>::VERSIONS
    }

    fn migrate(
        &self,
        segment: MessageSegment,
        receiver: Role,
        type_id: u32,
        body: &[u8],
        read_minor: u16,
        write_minor: u16,
    ) -> Result<(u32, Vec<u8>), ErrorCode> {
        let mut reader = BinaryReader::new(body).with_minor(read_minor);
        let mut writer = BinaryWriter::new();

        writer.set_minor(write_minor);

        let type_id =
            Migration::migrate(self, segment, receiver, type_id, &mut reader, &mut writer)?;

        reader.finish().map_err(|_| ErrorCode::MalformedMessage)?;

        Ok((type_id, writer.into_bytes()))
    }

    fn older_message(&self, receiver: Role, type_id: u32) -> Option<u32> {
        Migration::older_message(self, receiver, type_id)
    }
}

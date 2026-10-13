use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, Fixed32};
use crate::connection_id::ConnectionId;
use crate::definition::{self, Role};
use crate::error::Error;

const NO_ALIAS: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Route {
    pub(crate) from: Role,
    pub(crate) channel: u32,
    pub(crate) connection: Option<ConnectionId>,
}

impl Route {
    fn write(self, writer: &mut BinaryWriter) -> Result<(), Error> {
        definition::Route {
            from: self.from,
            channel: Fixed32(self.channel),
            connection: Fixed32(ConnectionId::to_u32(self.connection)),
        }
        .write(writer)
    }

    pub(crate) fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        let route = definition::Route::read(reader)?;

        Ok(Self {
            from: route.from,
            channel: route.channel.0,
            connection: ConnectionId::of(route.connection.0),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SliceKey {
    NoAlias(Route),
    Announcement(u32, Route),
    Alias(u32),
}

impl SliceKey {
    pub(crate) fn write(self, writer: &mut BinaryWriter) -> Result<(), Error> {
        match self {
            Self::NoAlias(route) => {
                writer.u32(NO_ALIAS);

                route.write(writer)
            }
            Self::Announcement(alias, route) => {
                writer.u32(alias * 2 - 1);

                route.write(writer)
            }
            Self::Alias(alias) => {
                writer.u32(alias * 2);

                Ok(())
            }
        }
    }

    pub(crate) fn read(reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        Ok(match reader.u32()? {
            NO_ALIAS => Self::NoAlias(Route::read(reader)?),
            key if key % 2 == 1 => Self::Announcement(key.div_ceil(2), Route::read(reader)?),
            key => Self::Alias(key / 2),
        })
    }
}

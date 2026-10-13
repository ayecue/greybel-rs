use crate::binary::{BinaryDecode, BinaryEncode, BinaryReader, BinaryWriter, Fixed32};
use crate::connection_id::ConnectionId;
use crate::definition::{Ack, Close, ErrorCode, FrameKind, Open, Refuse, Version};
use crate::error::{DecodeFailure, Error};

const ERR_NO_CONTROL: DecodeFailure = DecodeFailure::MessageAsControl;
const ERR_NO_CONNECTION: DecodeFailure = DecodeFailure::EmptyConnection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Control {
    Listen,
    Open {
        id: u32,
        core: Version,
        protocol: Version,
        fingerprint: u32,
        payload: Vec<u8>,
    },
    Ack {
        id: u32,
        connection: ConnectionId,
        core: Version,
        protocol: Version,
        fingerprint: u32,
        deprecation: Option<String>,
        payload: Vec<u8>,
    },
    Refuse {
        id: u32,
        reason: ErrorCode,
        core: Version,
        protocol: Version,
        majors: Vec<u16>,
        payload: Vec<u8>,
    },
    Ping,
    Pong,
    Close {
        reason: ErrorCode,
    },
}

impl Control {
    pub(crate) fn kind(&self) -> FrameKind {
        match self {
            Self::Listen => FrameKind::Listen,
            Self::Open { .. } => FrameKind::Open,
            Self::Ack { .. } => FrameKind::Ack,
            Self::Refuse { .. } => FrameKind::Refuse,
            Self::Ping => FrameKind::Ping,
            Self::Pong => FrameKind::Pong,
            Self::Close { .. } => FrameKind::Close,
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        matches!(self, Self::Listen | Self::Open { .. } | Self::Refuse { .. })
    }

    pub(crate) fn write(&self, writer: &mut BinaryWriter) -> Result<(), Error> {
        match self {
            Self::Listen | Self::Ping | Self::Pong => Ok(()),
            Self::Open {
                id,
                core,
                protocol,
                fingerprint,
                payload,
            } => {
                Open {
                    id: Fixed32(*id),
                    core: *core,
                    protocol: *protocol,
                    fingerprint: Fixed32(*fingerprint),
                }
                .write(writer)?;
                writer.raw(payload);

                Ok(())
            }
            Self::Ack {
                id,
                connection,
                core,
                protocol,
                fingerprint,
                deprecation,
                payload,
            } => {
                Ack {
                    id: Fixed32(*id),
                    connection: Fixed32(connection.get()),
                    core: *core,
                    protocol: *protocol,
                    fingerprint: Fixed32(*fingerprint),
                    deprecation: deprecation.clone(),
                }
                .write(writer)?;
                writer.raw(payload);

                Ok(())
            }
            Self::Refuse {
                id,
                reason,
                core,
                protocol,
                majors,
                payload,
            } => {
                Refuse {
                    id: Fixed32(*id),
                    reason: u32::from(*reason),
                    core: *core,
                    protocol: *protocol,
                    majors: majors.clone(),
                }
                .write(writer)?;
                writer.raw(payload);

                Ok(())
            }
            Self::Close { reason } => Close { reason: *reason }.write(writer),
        }
    }

    pub(crate) fn is_control(kind: FrameKind) -> bool {
        matches!(
            kind,
            FrameKind::Listen
                | FrameKind::Open
                | FrameKind::Ack
                | FrameKind::Refuse
                | FrameKind::Ping
                | FrameKind::Pong
                | FrameKind::Close
        )
    }

    pub(crate) fn read(kind: FrameKind, reader: &mut BinaryReader<'_>) -> Result<Self, Error> {
        Ok(match kind {
            FrameKind::Message
            | FrameKind::ResponseEnd
            | FrameKind::Failure
            | FrameKind::Cancel
            | FrameKind::ResponseStart
            | FrameKind::ResponseItem
            | FrameKind::ResponseCredit
            | FrameKind::MessageItem
            | FrameKind::MessageEnd
            | FrameKind::MessageCredit
            | FrameKind::Fragment => {
                return Err(Error::Decode(ERR_NO_CONTROL));
            }
            FrameKind::Listen => Self::Listen,
            FrameKind::Open => {
                let Open {
                    id,
                    core,
                    protocol,
                    fingerprint,
                } = Open::read(reader)?;

                Self::Open {
                    id: id.0,
                    core,
                    protocol,
                    fingerprint: fingerprint.0,
                    payload: reader.take_all().to_vec(),
                }
            }
            FrameKind::Ack => {
                let Ack {
                    id,
                    connection,
                    core,
                    protocol,
                    fingerprint,
                    deprecation,
                } = Ack::read(reader)?;

                Self::Ack {
                    id: id.0,
                    connection: ConnectionId::of(connection.0)
                        .ok_or(Error::Decode(ERR_NO_CONNECTION))?,
                    core,
                    protocol,
                    fingerprint: fingerprint.0,
                    deprecation,
                    payload: reader.take_all().to_vec(),
                }
            }
            FrameKind::Refuse => {
                let Refuse {
                    id,
                    reason,
                    core,
                    protocol,
                    majors,
                } = Refuse::read(reader)?;

                Self::Refuse {
                    id: id.0,
                    reason: ErrorCode::try_from(reason).unwrap_or(ErrorCode::VersionMismatch),
                    core,
                    protocol,
                    majors,
                    payload: reader.take_all().to_vec(),
                }
            }
            FrameKind::Ping => Self::Ping,
            FrameKind::Pong => Self::Pong,
            FrameKind::Close => Self::Close {
                reason: Close::read(reader)?.reason,
            },
        })
    }
}

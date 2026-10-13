use super::{ConnectionContext, Effect, NegotiatedOpen, Side, VersionTable};
use crate::binary::BinaryDecode;
use crate::connection_id::ConnectionId;
use crate::definition::{CORE, ErrorCode, Version};
use crate::frame::Control;
use crate::migration::{MigrationPath, Session};
use crate::protocol::{Protocol, VersionStatus};

const RANDOM_WORD_BYTES: usize = 4;
const MULTIPLIER: u32 = 0x93d7_65dd;
const ROTATE: u32 = 15;

impl<Pr: Protocol> ConnectionContext<Pr> {
    pub(crate) fn negotiate(
        &mut self,
        id: u32,
        core: Version,
        protocol: Version,
        fingerprint: u32,
        payload: &[u8],
        owner: Option<u32>,
    ) {
        let Some((versions, path)) = self.versions_and_path_of(protocol.major) else {
            return self.refuse(id, ErrorCode::VersionMismatch, Vec::new());
        };

        if let Some(reason) = Self::refusal_reason(core, protocol, fingerprint, versions) {
            return self.refuse(id, reason, Vec::new());
        }

        if let Some(owner) = owner {
            if owner != id {
                return self.refuse(id, ErrorCode::Busy, Vec::new());
            }

            if let Side::Provider { ack: Some(ack), .. } = &self.side {
                let ack = ack.clone();

                self.post(ack, None);
            }

            return;
        }

        let Side::Provider {
            negotiated_open, ..
        } = &mut self.side
        else {
            return;
        };

        if negotiated_open.is_some() {
            return;
        }

        let deprecation = match versions.status(protocol.minor) {
            VersionStatus::Deprecation(note) => Some((protocol, note.to_owned())),
            _ => None,
        };
        let protocol = protocol.lower_minor_version(versions.version);

        *negotiated_open = Some(NegotiatedOpen {
            id,
            fingerprint: versions.fingerprint(protocol.minor).unwrap_or_default(),
            session: Session {
                core: CORE.lower_minor_version(core),
                protocol,
                migration_path: path,
            },
            deprecation,
        });

        match Pr::Open::from_bytes(payload) {
            Ok(open) => self.emit(Effect::Open { id, open }),
            Err(_) => {
                if let Side::Provider {
                    negotiated_open, ..
                } = &mut self.side
                {
                    negotiated_open.take();
                }

                self.refuse(id, ErrorCode::MalformedMessage, Vec::new());
            }
        }
    }

    pub(crate) fn versions_and_path_of(
        &self,
        major: u16,
    ) -> Option<(VersionTable, Option<MigrationPath>)> {
        let native = self.versions.version.major;

        if major == native {
            return Some((self.versions, None));
        }

        let Side::Provider { migrations, .. } = &self.side else {
            return None;
        };
        let path = migrations.path(major, native)?;

        Some((path.versions(), Some(path)))
    }

    pub(crate) fn refusal_reason(
        core: Version,
        protocol: Version,
        fingerprint: u32,
        versions: VersionTable,
    ) -> Option<ErrorCode> {
        if core.major != CORE.major {
            return Some(ErrorCode::VersionMismatch);
        }

        if protocol.minor <= versions.version.minor
            && versions.fingerprint(protocol.minor) != Some(fingerprint)
        {
            return Some(ErrorCode::VersionMismatch);
        }

        if versions.status(protocol.minor) == VersionStatus::Unavailable {
            return Some(ErrorCode::UnsupportedVersion);
        }

        None
    }

    pub(crate) fn refuse(&mut self, id: u32, reason: ErrorCode, payload: Vec<u8>) {
        let native = self.versions.version.major;
        let majors = match &self.side {
            Side::Provider { migrations, .. } => migrations.majors(native),
            Side::Client { .. } => vec![native],
        };
        let refuse = Control::Refuse {
            id,
            reason,
            core: CORE,
            protocol: self.versions.version,
            majors,
            payload,
        };

        self.post(refuse, None);
    }

    pub(crate) fn connection_id(random: [u8; 16]) -> ConnectionId {
        let hash = random
            .as_chunks::<RANDOM_WORD_BYTES>()
            .0
            .iter()
            .fold(0u32, |hash, word| {
                hash.wrapping_add(u32::from_le_bytes(*word))
                    .wrapping_mul(MULTIPLIER)
            });

        ConnectionId::of(hash.rotate_left(ROTATE)).unwrap_or(ConnectionId::FIRST)
    }
}

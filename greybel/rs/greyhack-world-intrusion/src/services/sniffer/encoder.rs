//! Saving the encoder a sniffer captured (`HelperScript.GetPendingSniffers`).

use greyhack_behaviour::{FileData, PermissionSet, path};
use greyhack_world_core::{ComputerId, FileFailure, NodeDraft, Result};

use crate::{SnifferPacket, Sniffers};

/// Why a sniffed encoder could not be saved (`HelperScript.GetPendingSniffers`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncoderFailure {
    /// The user cannot write the `encode.src` at `path`.
    NoPermission {
        /// Path of the encoder file.
        path: String,
    },
    /// Another file failure.
    File(FileFailure),
}

/// Methods of [`World`] for intrusion sniffer encoder.
impl Sniffers {
    pub async fn save_packet_encoder(
        &self,
        computer: ComputerId,
        packet: &SnifferPacket,
        program_path: &str,
        user: &str,
    ) -> Result<Option<EncoderFailure>> {
        let Some(encoder) = &packet.encoder else {
            return Ok(None);
        };

        Ok(self
            .save_encoder(computer, program_path, user, encoder)
            .await?
            .err()
            .map(|failure| match failure {
                FileFailure::NoPermission => EncoderFailure::NoPermission {
                    path: path::join(path::parent(program_path), "encode.src"),
                },
                failure => EncoderFailure::File(failure),
            }))
    }

    pub async fn save_encoder(
        &self,
        computer: ComputerId,
        program: &str,
        user: &str,
        source: &str,
    ) -> Result<Result<(), FileFailure>> {
        let folder = path::parent(program);

        if !self.computers.has_computer(computer).await? {
            return Ok(Err(FileFailure::ComputerUnavailable));
        }

        let Some(folder_node) = self.files.lookup.folder(computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned())));
        };
        let path = path::join(folder, "encode.src");

        if let Some(mut file) = self.files.lookup.file(computer, &path, true).await? {
            if !self
                .files
                .permissions
                .has_node_permission(
                    &file,
                    greyhack_behaviour::Permission::Write,
                    self.computers.users.user(computer, user).await?.as_ref(),
                )
                .await?
            {
                return Ok(Err(FileFailure::NoPermission));
            }

            if let Some(data) = file.data_mut() {
                data.set_content(source);
            }

            file.timestamp = self.world.time_seconds();
            self.files.changes.save_node(&file).await?;

            return Ok(Ok(()));
        }

        self.files
            .changes
            .add_node(
                &folder_node,
                &NodeDraft::file(
                    "encode.src",
                    user,
                    PermissionSet::default_for(false),
                    FileData::text(source),
                ),
            )
            .await?;

        Ok(Ok(()))
    }
}

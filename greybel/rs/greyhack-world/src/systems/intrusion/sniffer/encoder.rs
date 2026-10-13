//! Saving the encoder a sniffer captured (`HelperScript.GetPendingSniffers`).

use greyhack_behaviour::{FileData, PermissionSet, path};

use crate::model::{ComputerId, NodeDraft, SnifferPacket};
use crate::{FileFailure, Result, World};

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

impl World {
    /// Saves the encoder of sniffed `packet` as `encode.src` beside `program_path` as `user`
    /// (`HelperScript.GetPendingSniffers`). Returns `None` when there is nothing to save or it saved.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn save_packet_encoder(
        &mut self,
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

    /// Writes `source` as `encode.src` beside `program` on `computer` as `user`.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why the file cannot be written.
    pub async fn save_encoder(
        &mut self,
        computer: ComputerId,
        program: &str,
        user: &str,
        source: &str,
    ) -> Result<Result<(), FileFailure>> {
        let folder = path::parent(program);

        if !self.db.has_computer(computer).await? {
            return Ok(Err(FileFailure::ComputerUnavailable));
        }

        let Some(folder_node) = self.folder(computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned())));
        };
        let path = path::join(folder, "encode.src");

        if let Some(mut file) = self.file(computer, &path, true).await? {
            if !self
                .has_node_permission(
                    &file,
                    greyhack_behaviour::Permission::Write,
                    self.user(computer, user).await?.as_ref(),
                )
                .await?
            {
                return Ok(Err(FileFailure::NoPermission));
            }

            if let Some(data) = file.data_mut() {
                data.set_content(source);
            }

            file.timestamp = self.time_seconds;
            self.save_node(&file).await?;

            return Ok(Ok(()));
        }

        self.add_node(
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

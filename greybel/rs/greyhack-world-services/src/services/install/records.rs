//! Service records and the viewer of them (`FileHelperServer.LeerFichaServerRpc`).

use greyhack_behaviour::{FileData, PermissionSet, ServiceId, path};
use greyhack_world_core::{ComputerId, Result};

use crate::services::InstallService;
use crate::{Record, RecordDatabase};

/// Service record-update errors (`FileHelperServer.UpdateFichaServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordFailure {
    /// The service records cannot be read.
    Unreadable,
    /// The record no longer exists.
    MissingEntry,
}

/// Methods of [`World`] for services install records.
impl InstallService {
    pub async fn service_records(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> Result<Option<RecordDatabase>> {
        if !matches!(
            service,
            ServiceId::Criminals | ServiceId::Employees | ServiceId::Students
        ) {
            return Ok(None);
        }

        Ok(self
            .files
            .lookup
            .content(computer, service.config_path(), false)
            .await?
            .and_then(|content| RecordDatabase::parse(&content)))
    }

    pub async fn update_service_record(
        &self,
        computer: ComputerId,
        service: ServiceId,
        record: Record,
    ) -> Result<Result<(), RecordFailure>> {
        let Some(mut database) = self.service_records(computer, service).await? else {
            return Ok(Err(RecordFailure::Unreadable));
        };

        if !database.update(record) {
            return Ok(Err(RecordFailure::MissingEntry));
        }

        self.files
            .changes
            .write_file(
                computer,
                service.config_path(),
                FileData::text(&database.to_json()),
            )
            .await?;

        Ok(Ok(()))
    }

    pub async fn write_launcher(
        &self,
        computer: ComputerId,
        path: &str,
        target: &str,
    ) -> Result<()> {
        let mut data = FileData::binary(FileData::binary_size(path::file_name(path), false));

        target.clone_into(&mut data.content);
        path::file_name(path).clone_into(&mut data.command_name);
        self.files.changes.write_file(computer, path, data).await?;
        if let Some(mut launcher) = self.files.lookup.file(computer, path, false).await? {
            launcher.permissions =
                PermissionSet::parse("-r-xr-xr-x").expect("launcher permissions");
            self.files.changes.save_node(&launcher).await?;
        }

        Ok(())
    }
}

//! Service records and the viewer of them (`FileHelperServer.LeerFichaServerRpc`).

use greyhack_behaviour::{FileData, PermissionSet, ServiceId, path};

use crate::{ComputerId, Record, RecordDatabase, Result, World};

/// Service record-update errors (`FileHelperServer.UpdateFichaServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordFailure {
    /// The service records cannot be read.
    Unreadable,
    /// The record no longer exists.
    MissingEntry,
}

impl World {
    /// Reads a SQL records database exposed by `FileHelperServer.LeerFichaServerRpc`. Missing or
    /// malformed databases return `None`.
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
            .content(computer, service.config_path(), false)
            .await?
            .and_then(|content| RecordDatabase::parse(&content)))
    }

    /// Replaces the record with the same `person_id` (`FileHelperServer.UpdateFichaServerRpc`) or
    /// returns the update failure.
    pub async fn update_service_record(
        &mut self,
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

        self.write_file(
            computer,
            service.config_path(),
            FileData::text(&database.to_json()),
        )
        .await?;

        Ok(Ok(()))
    }

    /// Writes an executable SQL-record viewer containing its server `localIp:port`
    /// (`Computer.ConfigFichaViewer`).
    pub(crate) async fn write_launcher(
        &mut self,
        computer: ComputerId,
        path: &str,
        target: &str,
    ) -> Result<()> {
        let mut data = FileData::binary(FileData::binary_size(path::file_name(path), false));

        data.content = target.to_owned();
        data.command_name = path::file_name(path).to_owned();
        self.write_file(computer, path, data).await?;
        if let Some(mut launcher) = self.file(computer, path, false).await? {
            launcher.permissions =
                PermissionSet::parse("-r-xr-xr-x").expect("launcher permissions");
            self.save_node(&launcher).await?;
        }

        Ok(())
    }
}

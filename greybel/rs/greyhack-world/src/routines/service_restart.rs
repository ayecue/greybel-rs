//! Service restart routine ([`Routine::ServiceRestart`](super::Routine::ServiceRestart)).

use greyhack_behaviour::{FIRST_VERSION, ServiceId};

use super::{RoutineFailure, TickReport};
use crate::{ComputerId, Result, World};

impl World {
    /// Restarts `service` on the computer at its current version.
    pub(super) async fn restart_service(
        &mut self,
        computer: ComputerId,
        service: ServiceId,
        report: &mut TickReport,
    ) -> Result<()> {
        if !self.db.has_computer(computer).await? {
            report.errors.push(RoutineFailure::ServiceComputerGone);

            return Ok(());
        }

        let version = self
            .db
            .service_port(computer, service)
            .await?
            .map_or_else(|| FIRST_VERSION.to_owned(), |port| port.version);

        self.stop_service(computer, service).await?;
        self.start_service(computer, service, &version).await?;

        Ok(())
    }
}

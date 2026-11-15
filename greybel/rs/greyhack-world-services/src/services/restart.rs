//! Service-restart routine owned by the services domain.

use std::rc::Rc;

use greyhack_behaviour::{FIRST_VERSION, ServiceId};
use greyhack_world_core::{ComputerId, ComputerService, WorldRef};
use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{Code, execute, fetch_column, fetch_opt};
use turso::{Connection, params};

use crate::storage::schema;

/// A nonfatal failure produced by a service restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceRestartFailure {
    /// The service's computer no longer exists.
    ComputerMissing,
}

/// Service-restart failures produced during one scheduler call.
#[derive(Default)]
pub struct ServiceRestartReport {
    /// Failures to append to the world's tick report.
    pub errors: Vec<ServiceRestartFailure>,
}

/// A service restarts on a computer.
pub struct ServiceRestart {
    world: WorldRef,
    computers: greyhack_world_modules::Dep<ComputerService>,
    scheduler: Rc<Scheduler>,
}

impl ServiceRestart {
    pub(crate) fn new(
        world: &WorldRef,
        computers: greyhack_world_modules::Dep<ComputerService>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers,
            scheduler,
        }
    }

    /// Schedules a service restart.
    pub async fn schedule(
        &self,
        at: u64,
        computer: ComputerId,
        service: ServiceId,
    ) -> greyhack_world_core::Result<RoutineId> {
        Ok(self
            .scheduler
            .schedule::<Self>(&self.world.db().connection(), at, &(computer, service))
            .await?)
    }

    async fn restart(
        &self,
        computer: ComputerId,
        service: ServiceId,
    ) -> greyhack_world_core::Result<()> {
        if !self.computers.has_computer(computer).await? {
            self.world
                .world_mut()
                .state_mut::<ServiceRestartReport>()
                .errors
                .push(ServiceRestartFailure::ComputerMissing);

            return Ok(());
        }

        let version = self
            .computers
            .service_port(computer, service)
            .await?
            .map_or_else(|| FIRST_VERSION.to_owned(), |port| port.version);

        self.computers
            .processes
            .stop_service(computer, service)
            .await?;
        self.computers
            .processes
            .start_service(computer, service, &version)
            .await?;

        Ok(())
    }
}

impl Routine for ServiceRestart {
    type Data = (ComputerId, ServiceId);

    const KEY: &'static str = "services.service_restart";
    const PRIORITY: i64 = 0;

    fn tables() -> TableSet {
        schema::routines::SERVICE_RESTARTS
    }

    async fn store(
        &self,
        conn: &Connection,
        id: RoutineId,
        data: &Self::Data,
    ) -> RoutineResult<()> {
        let (computer, service) = data;

        execute(
            conn,
            "INSERT INTO routine_service_restarts (routine_id, computer_id, service) VALUES (?1, ?2, ?3)",
            params![id.get().cast_signed(), computer.0, service.code()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<Self::Data>> {
        let row = fetch_opt::<(u32, Code<ServiceId>)>(
            conn,
            "SELECT computer_id, service FROM routine_service_restarts WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(row.map(|(computer, Code(service))| (ComputerId(computer), service)))
    }

    async fn run(&self, _id: RoutineId, data: Self::Data) -> RoutineResult<()> {
        self.restart(data.0, data.1)
            .await
            .map_err(RoutineError::new)
    }

    async fn cancel_for_computers(
        &self,
        conn: &Connection,
        computers: &[u32],
    ) -> RoutineResult<()> {
        for computer in computers {
            let rows: Vec<i64> = fetch_column(
                conn,
                "SELECT routine_id FROM routine_service_restarts WHERE computer_id = ?1",
                params![computer],
            )
            .await?;

            for row in rows {
                let id = RoutineId::new(row.cast_unsigned());

                if self.scheduler.cancel(conn, id).await? {
                    self.world
                        .works()
                        .cancel(greyhack_world_effects::WorkId::from(id));
                }
            }
        }

        Ok(())
    }
}

//! Library-version check routine owned by the libraries domain.

use std::rc::Rc;

use greyhack_behaviour::library_check_seconds;
use greyhack_world_core::WorldRef;
use greyhack_world_routines::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_value};
use turso::{Connection, params};

use crate::VersionService;
use crate::storage::schema;

/// The check for library versions to release.
pub struct LibraryCheck {
    world: WorldRef,
    versions: greyhack_world_core::Dep<VersionService>,
    scheduler: Rc<Scheduler>,
}

impl LibraryCheck {
    pub(crate) fn new(
        world: &WorldRef,
        versions: greyhack_world_core::Dep<VersionService>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            versions,
            scheduler,
        }
    }

    /// Schedules the first version check when the world has none.
    pub async fn ensure(&self) -> crate::Result<()> {
        if !self.versions.library_versions.has_library_check().await? {
            self.schedule().await?;
        }

        Ok(())
    }

    async fn schedule(&self) -> crate::Result<()> {
        let at = self
            .world
            .time_seconds()
            .saturating_add(library_check_seconds());

        self.scheduler
            .schedule::<Self>(&self.world.db().connection(), at, &())
            .await?;

        Ok(())
    }
}

impl Routine for LibraryCheck {
    type Data = ();

    const KEY: &'static str = "libraries.version_check";
    const PRIORITY: i64 = 6;

    fn tables() -> TableSet {
        schema::LIBRARY_CHECKS
    }

    async fn store(&self, conn: &Connection, id: RoutineId, (): &()) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_library_checks (routine_id) VALUES (?1)",
            params![id.cast_signed()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<()>> {
        let row = fetch_value::<i64>(
            conn,
            "SELECT routine_id FROM routine_library_checks WHERE routine_id = ?1",
            params![id.cast_signed()],
        )
        .await?;

        Ok(row.map(|_| ()))
    }

    async fn run(&self, _id: RoutineId, (): ()) -> RoutineResult<()> {
        let result = async {
            self.versions
                .library_versions
                .clear_library_reporters()
                .await?;
            self.versions.check_library_versions().await?;

            self.schedule().await
        }
        .await;

        result.map_err(RoutineError::new)
    }
}

//! Daily coin-price upkeep owned by the economy domain.

use std::rc::Rc;

use greyhack_behaviour::SECONDS_PER_DAY;
use greyhack_world_core::WorldRef;
use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_value};
use turso::{Connection, params};

use crate::Market;
use crate::storage::schema;

/// Daily coin-price upkeep.
pub struct DailyUpkeep {
    world: WorldRef,
    market: greyhack_world_modules::Dep<Market>,
    scheduler: Rc<Scheduler>,
}

impl DailyUpkeep {
    pub(crate) fn new(
        world: &WorldRef,
        market: greyhack_world_modules::Dep<Market>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            market,
            scheduler,
        }
    }

    /// Runs upkeep when this world has none and schedules the next day.
    pub async fn ensure(&self) -> greyhack_world_core::Result<()> {
        let conn = self.world.db().connection();
        let row = fetch_value::<i64>(
            &conn,
            "SELECT routine_id FROM routine_daily_upkeep LIMIT 1",
            (),
        )
        .await?;

        if row.is_none() {
            self.execute().await?;
        }

        Ok(())
    }

    async fn schedule(&self, at: u64) -> greyhack_world_core::Result<()> {
        self.scheduler
            .schedule::<Self>(&self.world.db().connection(), at, &())
            .await?;

        Ok(())
    }

    async fn execute(&self) -> greyhack_world_core::Result<()> {
        self.market.daily_upkeep().await?;

        let now = self.world.time_seconds();
        let next = (now / SECONDS_PER_DAY + 1).saturating_mul(SECONDS_PER_DAY);

        self.schedule(next).await
    }
}

impl Routine for DailyUpkeep {
    type Data = ();

    const KEY: &'static str = "economy.daily_upkeep";
    const PRIORITY: i64 = -1;

    fn tables() -> TableSet {
        schema::DAILY_UPKEEP
    }

    async fn store(&self, conn: &Connection, id: RoutineId, (): &()) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_daily_upkeep (routine_id) VALUES (?1)",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<()>> {
        let row = fetch_value::<i64>(
            conn,
            "SELECT routine_id FROM routine_daily_upkeep WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(row.map(|_| ()))
    }

    async fn run(&self, _id: RoutineId, (): ()) -> RoutineResult<()> {
        self.execute().await.map_err(RoutineError::new)
    }
}

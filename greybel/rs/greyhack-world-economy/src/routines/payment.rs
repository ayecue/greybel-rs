//! Subscription-payment routine owned by the economy domain.

use std::rc::Rc;

use greyhack_behaviour::SubscriptionKind;
use greyhack_world_core::WorldRef;
use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{Code, execute, fetch_column, fetch_opt};
use turso::{Connection, params};

use crate::Bank;
use crate::storage::schema;

/// A subscription's payment falls due.
pub struct Payment {
    world: WorldRef,
    bank: greyhack_world_modules::Dep<Bank>,
    scheduler: Rc<Scheduler>,
}

impl Payment {
    pub(crate) fn new(
        world: &WorldRef,
        bank: greyhack_world_modules::Dep<Bank>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            bank,
            scheduler,
        }
    }

    pub(crate) async fn schedule_with(
        world: &WorldRef,
        scheduler: &Scheduler,
        at: u64,
        bank: &str,
        kind: SubscriptionKind,
    ) -> greyhack_world_core::Result<RoutineId> {
        Self::cancel_with(world, scheduler, bank, kind).await?;

        Ok(scheduler
            .schedule::<Self>(&world.db().connection(), at, &(bank.to_owned(), kind))
            .await?)
    }

    /// Replaces a subscription's payment timer.
    pub async fn schedule(
        &self,
        at: u64,
        bank: &str,
        kind: SubscriptionKind,
    ) -> greyhack_world_core::Result<RoutineId> {
        Self::schedule_with(&self.world, &self.scheduler, at, bank, kind).await
    }

    pub(crate) async fn cancel_with(
        world: &WorldRef,
        scheduler: &Scheduler,
        bank: &str,
        kind: SubscriptionKind,
    ) -> greyhack_world_core::Result<()> {
        let conn = world.db().connection();
        let rows: Vec<i64> = fetch_column(
            &conn,
            "SELECT routine_id FROM routine_payments WHERE bank_number = ?1 AND kind = ?2",
            params![bank, kind.code()],
        )
        .await?;

        for row in rows {
            scheduler
                .cancel(&conn, RoutineId::new(row.cast_unsigned()))
                .await?;
        }

        Ok(())
    }
}

impl Routine for Payment {
    type Data = (String, SubscriptionKind);

    const KEY: &'static str = "economy.payment";
    const PRIORITY: i64 = 5;

    fn tables() -> TableSet {
        schema::PAYMENTS
    }

    async fn store(
        &self,
        conn: &Connection,
        id: RoutineId,
        data: &Self::Data,
    ) -> RoutineResult<()> {
        let (bank, kind) = data;

        execute(
            conn,
            "INSERT INTO routine_payments (routine_id, bank_number, kind) VALUES (?1, ?2, ?3)",
            params![id.get().cast_signed(), bank.as_str(), kind.code()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<Self::Data>> {
        let row = fetch_opt::<(String, Code<SubscriptionKind>)>(
            conn,
            "SELECT bank_number, kind FROM routine_payments WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(row.map(|(bank, Code(kind))| (bank, kind)))
    }

    async fn run(&self, _id: RoutineId, data: Self::Data) -> RoutineResult<()> {
        self.bank
            .run_payment(&data.0, data.1)
            .await
            .map_err(RoutineError::new)
    }
}

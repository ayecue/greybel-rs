//! Script-driven DDoS floods that continue until reboot or cancellation (`FloodSystem`).

use greyhack_world_core::{ComputerId, Result};
use greyhack_world_effects::WorkId;

/// Flood start result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flood {
    /// The flood reached the reboot threshold: flood records are cleared and the computer reboots.
    Reboot,
    /// The computer remains online and the flood continues until its script stops.
    Active(WorkId),
    /// A CrownShield blocked the traffic, so nothing floods and the script waits until stopped
    /// (`NetSessionIntrinsics.flood_connection`).
    Shield,
}

/// Methods of [`World`] for intrusion flood.
impl FloodService {
    pub async fn flood(&self, computer: ComputerId, origin: &str, threshold: u32) -> Result<Flood> {
        let (work, counts_for_reboot) = self.subscribers.insert_flood(computer, origin).await?;

        if counts_for_reboot && self.subscribers.flood_count(computer).await? >= threshold {
            self.subscribers.clear_floods(computer).await?;
            self.computers.processes.reboot(computer, true).await?;

            return Ok(Flood::Reboot);
        }

        Ok(Flood::Active(work))
    }

    /// Ends flood `id` with `result` and reports whether it existed.
    pub async fn end<T: 'static>(&self, id: WorkId, result: T) -> Result<bool> {
        let present = self.subscribers.remove_flood(id).await?;

        if present {
            self.world.works().finish(id, result);
        }

        Ok(present)
    }

    /// Cancels flood `id` and reports whether it existed.
    pub async fn cancel(&self, id: WorkId) -> Result<bool> {
        let present = self.subscribers.remove_flood(id).await?;

        if present {
            self.world.works().cancel(id);
        }

        Ok(present)
    }
}

/// A world service with only its declared dependencies.
pub struct FloodService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) subscribers: std::rc::Rc<crate::storage::SubscriberRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
}

impl FloodService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        subscribers: std::rc::Rc<crate::storage::SubscriberRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
        guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    ) -> Self {
        Self {
            world: world.clone(),
            subscribers,
            computers,
            networks,
            jobs,
            guilds,
        }
    }
}

impl greyhack_world_effects::WorkOwner for FloodService {
    fn cancel(&self, id: WorkId) -> greyhack_world_effects::WorkCancelFuture<'_> {
        Box::pin(async move {
            FloodService::cancel(self, id)
                .await
                .map_err(greyhack_world_effects::EventError::new)
        })
    }
}

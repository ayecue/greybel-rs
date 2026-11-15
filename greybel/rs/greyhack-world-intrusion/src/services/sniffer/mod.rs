//! Sniffers: starting them, the paths they watch, the packets they record, and captured encoders.

pub(crate) mod encoder;
pub(crate) mod npc_logins;
pub(crate) mod packets;
pub(crate) mod path;

use greyhack_world_core::Result;
use greyhack_world_modules::WorkId;

/// A world service with only its declared dependencies.
pub struct SnifferService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) subscribers: std::rc::Rc<crate::storage::SubscriberRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
}

impl SnifferService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        subscribers: std::rc::Rc<crate::storage::SubscriberRepository>,
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
    ) -> Self {
        Self {
            world: world.clone(),
            subscribers,
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
        }
    }
}

impl SnifferService {
    /// Ends sniffer `id` with `result` and reports whether it existed.
    pub async fn end<T: 'static>(&self, id: WorkId, result: T) -> Result<bool> {
        let present = self.subscribers.remove_sniffer(id).await?;

        if present {
            self.world.works().finish(id, result);
        }

        Ok(present)
    }

    /// Cancels sniffer `id` and reports whether it existed.
    pub async fn cancel(&self, id: WorkId) -> Result<bool> {
        let present = self.subscribers.remove_sniffer(id).await?;

        if present {
            self.world.works().cancel(id);
        }

        Ok(present)
    }
}

impl greyhack_world_modules::WorkOwner for SnifferService {
    fn cancel(&self, id: WorkId) -> greyhack_world_modules::WorkCancelFuture<'_> {
        Box::pin(async move {
            SnifferService::cancel(self, id)
                .await
                .map_err(greyhack_world_modules::EventError::new)
        })
    }
}

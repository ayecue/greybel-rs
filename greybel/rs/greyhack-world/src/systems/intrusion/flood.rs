//! Script-driven DDoS floods that continue until reboot or cancellation (`FloodSystem`).

use crate::world::WorkId;
use crate::{ComputerId, Result, World};

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

impl World {
    /// Floods `computer` from public `origin`; `threshold` concurrent origins trigger reboot
    /// (`FloodSystem.AddFlood`). Repeated floods from one origin do not increase the count, and
    /// stopping one removes the first origin record (`FloodSystem.RemoveFlood`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn flood(
        &mut self,
        computer: ComputerId,
        origin: &str,
        threshold: u32,
    ) -> Result<Flood> {
        let (work, counts_for_reboot) = self.db.insert_flood(computer, origin).await?;

        if counts_for_reboot && self.db.flood_count(computer).await? >= threshold {
            self.db.clear_floods(computer).await?;
            self.reboot(computer, true).await?;

            return Ok(Flood::Reboot);
        }

        Ok(Flood::Active(work))
    }
}

//! Script floods of a computer from another computer's address (`NetSessionIntrinsics.flood_connection`).

use greyhack_behaviour::{DEFAULT_NET_SPEED, flood_reboot_threshold};

use crate::{ComputerId, Flood, Result, World};

/// Warning the game prints before a flood starts (`NetSessionIntrinsics.flood_connection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloodWarning {
    /// A super administrator needs more traffic to reboot the computer.
    SuperAdmin,
    /// A CrownShield blocks the traffic.
    CrownShield,
}

/// A flood started from a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloodAttempt {
    /// Public address of the flooded computer.
    pub address: String,
    /// Warning the game shows, if any.
    pub warning: Option<FloodWarning>,
    /// Result of the flood.
    pub flood: Flood,
}

impl World {
    /// Floods `computer` from the public address of `origin` (`NetSessionIntrinsics.flood_connection`).
    /// Reboot occurs at `netSpeed × 3` concurrent sources with a super administrator, or 25% fewer
    /// without one. A company listed on the stock market that went down loses ranking points when a
    /// guild owns it (`IdProcess.CompanyDdos`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn flood_from(
        &mut self,
        computer: ComputerId,
        origin: ComputerId,
    ) -> Result<FloodAttempt> {
        let address = self.public_ip(computer).await?;
        let origin = self.public_ip(origin).await?;
        let network = self.computer(computer).await?.map(|row| row.network);
        let main_router = match network {
            Some(network) => self.main_router(network).await?,
            None => None,
        };
        let (super_admin, net_speed) = match &main_router {
            Some(router) => (
                self.has_super_admin(router.network).await?,
                self.net_speed(router.id)
                    .await?
                    .unwrap_or(DEFAULT_NET_SPEED),
            ),
            None => (false, DEFAULT_NET_SPEED),
        };
        let shield = match &main_router {
            Some(router) => self.is_crown_shield_protected(&router.public_ip).await?,
            None => false,
        };
        let warning = if shield {
            Some(FloodWarning::CrownShield)
        } else if super_admin {
            Some(FloodWarning::SuperAdmin)
        } else {
            None
        };

        if shield {
            return Ok(FloodAttempt {
                address,
                warning,
                flood: Flood::Shield,
            });
        }

        let threshold = flood_reboot_threshold(net_speed, super_admin);
        let flood = self.flood(computer, &origin, threshold).await?;

        if flood == Flood::Reboot
            && let Some(router) = &main_router
            && self.is_listed_on_stock_market(router.network).await?
        {
            self.apply_ddos_leaderboard_penalty(&router.public_ip)
                .await?;
        }

        Ok(FloodAttempt {
            address,
            warning,
            flood,
        })
    }
}

#[cfg(test)]
#[path = "tests/ddos.rs"]
mod tests;

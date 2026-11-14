//! Script floods of a computer from another computer's address (`NetSessionIntrinsics.flood_connection`).

use greyhack_behaviour::{DEFAULT_NET_SPEED, flood_reboot_threshold};
use greyhack_world_core::{ComputerId, Result};

use crate::{Flood, FloodService};

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

/// Methods of [`World`] for intrusion ddos.
impl FloodService {
    pub async fn flood_from(
        &self,
        computer: ComputerId,
        origin: ComputerId,
    ) -> Result<FloodAttempt> {
        let address = self.networks.topology.public_ip(computer).await?;
        let origin = self.networks.topology.public_ip(origin).await?;
        let network = self
            .networks
            .topology
            .computer(computer)
            .await?
            .map(|row| row.network);
        let main_router = match network {
            Some(network) => self.networks.topology.main_router(network).await?,
            None => None,
        };
        let (super_admin, net_speed) = match &main_router {
            Some(router) => (
                self.jobs.has_super_admin(router.network).await?,
                self.networks
                    .settings
                    .net_speed(router.id)
                    .await?
                    .unwrap_or(DEFAULT_NET_SPEED),
            ),
            None => (false, DEFAULT_NET_SPEED),
        };
        let shield = match &main_router {
            Some(router) => {
                self.guilds
                    .is_crown_shield_protected(&router.public_ip)
                    .await?
            }
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
            && self
                .guilds
                .is_listed_on_stock_market(router.network)
                .await?
        {
            self.guilds
                .apply_ddos_leaderboard_penalty(&router.public_ip)
                .await?;
        }

        Ok(FloodAttempt {
            address,
            warning,
            flood,
        })
    }
}

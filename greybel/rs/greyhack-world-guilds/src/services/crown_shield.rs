//! CrownShield protection of company networks (`NetworkHelperServer.ActivateCrownShieldProtection`).

use greyhack_behaviour::is_public_ip;

use crate::{ComputerId, CrownShieldFailure, GuildService, Result};

/// Methods of [`World`] for guilds crown shield.
impl GuildService {
    pub async fn is_crown_shield_protected(&self, public_ip: &str) -> Result<bool> {
        self.guilds.is_crown_shield_protected(public_ip).await
    }

    pub async fn is_crown_shield_network(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.computers.computer(computer).await? else {
            return Ok(false);
        };
        let Some(router) = self.networks.topology.main_router(row.network).await? else {
            return Ok(false);
        };

        self.guilds
            .is_crown_shield_protected(&router.public_ip)
            .await
    }

    pub async fn hire_crown_shield(
        &self,
        guild_name: &str,
        public_ip: &str,
    ) -> Result<std::result::Result<(), CrownShieldFailure>> {
        let Some(guild) = self.guilds.guild_by_name(guild_name).await? else {
            return Ok(Err(CrownShieldFailure::UnknownGuild(guild_name.to_owned())));
        };

        if !is_public_ip(public_ip) {
            return Ok(Err(CrownShieldFailure::InvalidAddress));
        }

        if !guild.has_crown {
            return Ok(Err(CrownShieldFailure::NoCrown));
        }

        if !guild.crown_shield_public_ip.is_empty() {
            return Ok(Err(CrownShieldFailure::AlreadyActive));
        }

        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(Err(CrownShieldFailure::UnknownNetwork(
                public_ip.to_owned(),
            )));
        };

        self.guilds.set_guild_shield(guild.id, public_ip).await?;
        self.guilds
            .add_crown_shield_network(public_ip, &guild.name)
            .await?;
        self.world
            .world_mut()
            .state_mut::<crate::CrownRank>()
            .mark_dirty();

        self.world.events().publish(crate::FloodClear {
            computer: router.id.computer(),
        });
        self.computers
            .processes
            .reboot(router.id.computer(), false)
            .await?;

        for computer in self.computers.computers_in(router.network).await? {
            self.world.events().publish(crate::FloodClear {
                computer: computer.id,
            });

            if computer.id != router.id.computer() && !self.world.is_player(computer.id) {
                self.computers.processes.reboot(computer.id, false).await?;
            }
        }

        Ok(Ok(()))
    }
}

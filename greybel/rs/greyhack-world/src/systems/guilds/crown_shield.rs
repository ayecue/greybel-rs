//! CrownShield protection of company networks (`NetworkHelperServer.ActivateCrownShieldProtection`).

use greyhack_behaviour::is_public_ip;

use crate::{ComputerId, CrownShieldFailure, Result, World};

impl World {
    /// Returns whether a CrownShield protects the network at `public_ip`
    /// (`Database.IsCrownShieldProtected`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_crown_shield_protected(&self, public_ip: &str) -> Result<bool> {
        self.db.is_crown_shield_protected(public_ip).await
    }

    /// Returns whether a CrownShield protects `computer`'s network.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_crown_shield_network(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.db.computer(computer).await? else {
            return Ok(false);
        };
        let Some(router) = self.main_router(row.network).await? else {
            return Ok(false);
        };

        self.db.is_crown_shield_protected(&router.public_ip).await
    }

    /// Hires CrownShield for the guild named `guild_name` at the network at `public_ip`: the guild
    /// protects it and every computer of it restarts and drops its floods
    /// (`NetworkHelperServer.ActivateCrownShieldProtection`, `Database.AddCrownShieldNetwork`).
    /// The price (`Guild.CrownShieldPrice`) is not charged.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn hire_crown_shield(
        &mut self,
        guild_name: &str,
        public_ip: &str,
    ) -> Result<std::result::Result<(), CrownShieldFailure>> {
        let Some(guild) = self.db.guild_by_name(guild_name).await? else {
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

        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(Err(CrownShieldFailure::UnknownNetwork(
                public_ip.to_owned(),
            )));
        };

        self.db.set_guild_shield(guild.id, public_ip).await?;
        self.db
            .add_crown_shield_network(public_ip, &guild.name)
            .await?;
        self.crown_ranking_dirty = true;

        self.db.clear_floods(router.id.computer()).await?;
        self.reboot(router.id.computer(), false).await?;

        for computer in self.db.computers_in(router.network).await? {
            self.db.clear_floods(computer.id).await?;

            if computer.id != router.id.computer() && !self.is_player(computer.id) {
                self.reboot(computer.id, false).await?;
            }
        }

        Ok(Ok(()))
    }
}

#[cfg(test)]
#[path = "tests/crown_shield.rs"]
mod tests;

//! Sessions of computer accounts and who is active on a computer.

use greyhack_behaviour::{GUEST_USER, ROOT_USER, is_system_process};

use crate::{ComputerId, Result, World};

impl World {
    /// Returns users with open sessions on `computer` in login order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn sessions(&self, computer: ComputerId) -> Result<Vec<String>> {
        self.db.sessions(computer).await
    }

    /// Opens a session for `user` on `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn open_session(&mut self, computer: ComputerId, user: &str) -> Result<()> {
        self.db.insert_session(computer, user).await
    }

    /// Returns whether the network administrator is logged in, making them active root on every
    /// non-player-created network computer (`Computer.GetAdmin`, `Computer.IsSpawnedByPlayer`,
    /// `Persona.IsLogged`). Requires being at work, alive, and not fired.
    async fn is_admin_active(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.db.computer(computer).await? else {
            return Ok(false);
        };

        if row.network_kind.is_player_made() {
            return Ok(false);
        }

        let Some(person) = self.db.network_admin_person(row.network).await? else {
            return Ok(false);
        };

        Ok(self
            .db
            .npc(person)
            .await?
            .is_some_and(|admin| admin.online && !admin.is_dead_or_fired(self.time_seconds)))
    }

    /// Returns whether any user is active on `computer` (`Computer.IsAnyActiveUser`): logged-in
    /// administrator, open session, or running user process.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_any_active_user(&self, computer: ComputerId) -> Result<bool> {
        Ok(self.is_admin_active(computer).await?
            || !self.db.sessions(computer).await?.is_empty()
            || self
                .db
                .processes(computer)
                .await?
                .iter()
                .any(|process| !is_system_process(&process.name)))
    }

    /// Returns whether root is active on `computer` (`Computer.IsRootActiveUser`): logged-in
    /// administrator, root session, or root process.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_root_active_user(&self, computer: ComputerId) -> Result<bool> {
        Ok(self.is_admin_active(computer).await?
            || self
                .db
                .sessions(computer)
                .await?
                .iter()
                .any(|user| user == ROOT_USER)
            || self
                .db
                .processes(computer)
                .await?
                .iter()
                .any(|process| !is_system_process(&process.name) && process.user == ROOT_USER))
    }

    /// Returns whether a guest-owned process runs on `computer` (`MetaLibIntrinsics`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_guest_active(&self, computer: ComputerId) -> Result<bool> {
        Ok(self
            .db
            .processes(computer)
            .await?
            .iter()
            .any(|process| process.user == GUEST_USER))
    }

    /// Opens a session for `user` on `computer` unless the user is missing or already has one open.
    /// Returns whether it opened.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn open_new_session(&mut self, computer: ComputerId, user: &str) -> Result<bool> {
        if self.user(computer, user).await?.is_none()
            || self
                .sessions(computer)
                .await?
                .iter()
                .any(|name| name == user)
        {
            return Ok(false);
        }

        self.open_session(computer, user).await?;

        Ok(true)
    }
}

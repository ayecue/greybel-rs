//! Rebooting a computer: the reboot itself, the player's restart, and the script request.

use greyhack_behaviour::{
    ROOT_USER, SAFE_MODE_TERMINAL, SAFE_MODE_TERMINAL_RAM_MB, is_system_process, reboot_seconds,
};

use crate::systems::ProcessService as ProcessesService;
use crate::systems::computer::failure::ComputerFailure;
use crate::world::RebootEvent;
use crate::{ComputerId, Process, Result};

impl ProcessesService {
    /// Reboots `computer` until world second `until` (`Computer.Reboot`), ending processes and
    /// sessions. Player-protected `kernel_task` and `Xorg` survive
    /// (`PlayerComputer.RemoveAllProcess`). Files, ports, and network state remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn reboot_computer(&self, computer: ComputerId, until: u64) -> Result<()> {
        let Some(mut row) = self.computer_rows.computer(computer).await? else {
            return Ok(());
        };

        let system_processes = if self.world.is_player(computer) {
            self.computer_rows
                .processes(computer)
                .await?
                .into_iter()
                .filter(|process| process.protected && is_system_process(&process.name))
                .collect()
        } else {
            Vec::new()
        };

        self.world.world_mut().process_ends.insert(computer);
        self.computer_rows.clear_processes(computer).await?;

        for process in system_processes {
            self.computer_rows
                .insert_process(computer, &process)
                .await?;
        }

        row = self.computer_rows.computer(computer).await?.unwrap_or(row);
        row.reboot_end = until;

        self.computer_rows.save_computer(&row).await
    }

    /// Reboots `computer` for [`reboot_seconds`] (`ConfigOS.Reboot`). Returns whether it exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn reboot(&self, computer: ComputerId, flooding: bool) -> Result<bool> {
        let until = self.world.time_seconds() + reboot_seconds(flooding);

        if !self.computer_rows.has_computer(computer).await? {
            return Ok(false);
        }

        self.reboot_computer(computer, until).await?;
        self.world.events().publish(RebootEvent { computer });

        Ok(true)
    }

    /// Restarts the player's computer from a terminal connected to it and runs what its client does
    /// when it starts again (`PlayerComputer.Reboot`). `PlayerHelperServer.CheckIniErrorServerRpc`
    /// starts the protected `Terminal-safemode` terminal instead of the desktop when `safe_mode` is
    /// set or the computer fails its startup checks; the desktop connects to the most recent saved
    /// Wi-Fi network (`BootUp.MostrarDesktopIcons`) when Wi-Fi is the active card. Returns whether
    /// the computer exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn reboot_player(&self, safe_mode: bool) -> Result<bool> {
        let player = self.world.player();

        if !self.reboot(player, false).await? {
            return Ok(false);
        }

        if safe_mode || !self.startup.startup_failures(player).await?.is_empty() {
            self.computer_rows
                .spawn_process_as(
                    player,
                    &Process {
                        pid: 0,
                        user: ROOT_USER.to_owned(),
                        name: SAFE_MODE_TERMINAL.to_owned(),
                        cpu: 0.0,
                        ram_mb: SAFE_MODE_TERMINAL_RAM_MB,
                        protected: true,
                        service: None,
                        is_terminal: true,
                    },
                )
                .await?;
        } else if self
            .net_cards
            .active_net_card(player)
            .await?
            .is_some_and(|card| card.wifi)
        {
            // The game tells the client's network window why the connection was refused; the
            // computer stays where it is.
            let _ = self.net_cards.auto_connect_wifi().await?;
        }

        Ok(true)
    }

    /// Returns whether `user` on `computer` is root: the account is root, or is in the root group
    /// (`ConfigOS`).
    async fn has_root_rights(&self, computer: ComputerId, user: &str) -> Result<bool> {
        Ok(self.users.user(computer, user).await?.is_some_and(|user| {
            user.is_root() || user.groups.iter().any(|group| group == ROOT_USER)
        }))
    }

    /// Reboots `computer` as `user` (`ComputerIntrinsics.reboot`). Requires root or root-group
    /// access, and safe mode only on the player's computer. The player's computer starts again as
    /// its client does when the script's terminal is connected to it (`Terminal.RebootingSystem`):
    /// in safe mode, or with the desktop connecting to the most recent saved Wi-Fi network. Returns
    /// `false` if the computer no longer exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn reboot_as(
        &self,
        computer: ComputerId,
        user: &str,
        terminal_computers: &[ComputerId],
        safe_mode: bool,
    ) -> Result<Result<bool, ComputerFailure>> {
        if !self.has_root_rights(computer, user).await? {
            return Ok(Err(ComputerFailure::NoRebootPermission));
        }

        if computer != self.world.player() && safe_mode {
            return Ok(Err(ComputerFailure::RemoteSafeMode));
        }

        let player = self.world.player();
        let at_player = computer == player
            && terminal_computers
                .last()
                .is_none_or(|computer| *computer == player);

        Ok(Ok(if at_player {
            self.reboot_player(safe_mode).await?
        } else {
            self.reboot(computer, false).await?
        }))
    }
}

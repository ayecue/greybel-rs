//! Reverse shells of the rshell service (`MetaExploitIntrinsics`, `ShellIntrinsics`).

use greyhack_behaviour::{NetworkKind, Refusal, ServiceId, check_process_name};
use greyhack_world_core::systems::network::connect::ServiceCheck;
use greyhack_world_core::{ComputerId, Result};

use crate::{Metaxploit, ReverseShell, RshellFailure};

/// Why starting a reverse-shell client failed (`MetaExploitIntrinsics.rshell_client`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RshellClientFailure {
    /// The process name breaks an argument rule.
    Refusal(Refusal),
    /// The connection to the server failed.
    Connection(RshellFailure),
}

/// Why reverse shells cannot be listed (`MetaExploitIntrinsics.rshell_server`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RshellServerFailure {
    /// The computer does not run the rshell service.
    NoService,
    /// The router has no forward to the computer's rshell service.
    ForwardMissing,
}

/// Why reverse shells cannot be removed (`MetaExploitIntrinsics.rshell_remove`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RshellRemoveFailure {
    /// The computer does not run the rshell service.
    NoService,
    /// The router has no forward to the computer's rshell service.
    ForwardMissing,
    /// The computer is on a CTF network.
    CtfNetwork,
    /// The shells argument was neither null nor a list.
    ExpectedList,
    /// A list item was not a reverse shell.
    OnlyReverseShells,
}

/// Reverse-shell client connected to a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RshellClient {
    /// Computer the client runs on.
    pub computer: ComputerId,
    /// User of the client process.
    pub user: String,
    /// Client process.
    pub pid: u32,
}

/// Methods of [`World`] for exploits metaxploit rshell.
impl Metaxploit {
    pub async fn connect_reverse_shell(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
        shell: &ReverseShell,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<(), RshellFailure>> {
        let Some(router) = self.networks.routers.router_at(address).await? else {
            return Ok(Err(RshellFailure::UnknownAddress));
        };
        // `Router.GetServidor`: resolves the computer targeted by the router port.
        let server = match self
            .networks
            .lan
            .port_table(router.id)
            .await?
            .into_iter()
            .find(|entry| entry.number == port)
        {
            Some(entry) => {
                self.networks
                    .topology
                    .device(router.network, &entry.lan_ip)
                    .await?
            }
            None => None,
        };
        let Some(server) = server else {
            return Ok(Err(RshellFailure::MissingServer));
        };
        let source = self.networks.topology.public_ip(from).await?;

        if let Some(failure) = self
            .networks
            .connect
            .check_service(&ServiceCheck {
                target: server,
                service: ServiceId::Rshell,
                source: &source,
                address,
                port,
                firewall: true,
                terminal_computers,
            })
            .await?
        {
            return Ok(Err(RshellFailure::Service(failure)));
        }

        if !self
            .reverse_shells
            .insert_reverse_shell(router.id, shell)
            .await?
        {
            return Ok(Err(RshellFailure::Limit));
        }

        Ok(Ok(()))
    }

    pub async fn start_reverse_shell(
        &self,
        computer: ComputerId,
        user: &str,
        address: &str,
        port: u16,
        process_name: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Option<Result<(), RshellClientFailure>>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if let Err(refusal) = check_process_name(process_name) {
            return Ok(Some(Err(RshellClientFailure::Refusal(refusal))));
        }

        let pid = self
            .computers
            .processes
            .spawn_process(computer, user, process_name, None)
            .await?;
        let shell = ReverseShell {
            computer,
            port,
            user: user.to_owned(),
            pid,
            creation_second: self.world.time_seconds(),
        };

        Ok(Some(
            self.connect_reverse_shell(computer, address, port, &shell, terminal_computers)
                .await?
                .map_err(RshellClientFailure::Connection),
        ))
    }

    pub async fn rshell_clients(
        &self,
        computer: ComputerId,
    ) -> Result<Option<std::result::Result<Vec<RshellClient>, RshellServerFailure>>> {
        let Some(computer) = self.networks.topology.computer(computer).await? else {
            return Ok(None);
        };

        if !self
            .computers
            .processes
            .has_service(computer.id, ServiceId::Rshell)
            .await?
        {
            return Ok(Some(Err(RshellServerFailure::NoService)));
        }

        let Some(router) = (match computer.router {
            Some(router) => self.networks.routers.router(router).await?,
            None => None,
        }) else {
            return Ok(None);
        };
        let forward = self
            .networks
            .router_config
            .port_forwards(router.id)
            .await?
            .into_iter()
            .find(|forward| {
                forward.port.service == ServiceId::Rshell && forward.lan_ip == computer.local_ip
            })
            .map(|forward| forward.number);
        let Some(forward) = forward else {
            return Ok(Some(Err(RshellServerFailure::ForwardMissing)));
        };
        let mut clients = Vec::new();

        for shell in self.reverse_shells.reverse_shells(router.id).await? {
            if shell.port != forward {
                continue;
            }

            let remote = self.networks.topology.computer(shell.computer).await?;
            let has_process = self
                .computers
                .processes
                .process(shell.computer, shell.pid)
                .await?
                .is_some();
            // `ConfigOS.ShouldBreakConnection`: rebooting the client after creation breaks its shell.
            let connected = remote.is_some_and(|computer| {
                has_process && shell.creation_second >= computer.reboot_end
            });
            let enabled = !self.networks.settings.is_disabled(shell.computer).await?;

            // `MetaExploitIntrinsics.rshell_server` removes every stale registration rather than only skipping it.
            if connected && enabled {
                clients.push(RshellClient {
                    computer: shell.computer,
                    user: shell.user,
                    pid: shell.pid,
                });
            } else {
                self.reverse_shells
                    .remove_reverse_shell(router.id, shell.computer, shell.pid)
                    .await?;
            }
        }

        Ok(Some(Ok(clients)))
    }

    pub async fn remove_rshell_clients(
        &self,
        computer: ComputerId,
        selection: std::result::Result<Option<&[RshellClient]>, RshellRemoveFailure>,
    ) -> Result<Option<std::result::Result<bool, RshellRemoveFailure>>> {
        let Some(computer) = self.networks.topology.computer(computer).await? else {
            return Ok(None);
        };

        if computer.network_kind == NetworkKind::Ctf {
            return Ok(Some(Err(RshellRemoveFailure::CtfNetwork)));
        }

        if !self
            .computers
            .processes
            .has_service(computer.id, ServiceId::Rshell)
            .await?
        {
            return Ok(Some(Err(RshellRemoveFailure::NoService)));
        }

        // The game validates the list after checking the service but before checking its port forward.
        let selection = match selection {
            Ok(selection) => selection,
            Err(failure) => return Ok(Some(Err(failure))),
        };
        let Some(router) = (match computer.router {
            Some(router) => self.networks.routers.router(router).await?,
            None => None,
        }) else {
            return Ok(None);
        };
        let forward = self
            .networks
            .router_config
            .port_forwards(router.id)
            .await?
            .into_iter()
            .find(|forward| {
                forward.port.service == ServiceId::Rshell && forward.lan_ip == computer.local_ip
            })
            .map(|forward| forward.number);
        let Some(forward) = forward else {
            return Ok(Some(Err(RshellRemoveFailure::ForwardMissing)));
        };
        let mut removed = false;

        // `RouterConfigOS.RemoveReverseShells`: null selects all on the port, otherwise matching netID and PID.
        for shell in self.reverse_shells.reverse_shells(router.id).await? {
            let selected = selection.is_none_or(|shells| {
                shells
                    .iter()
                    .any(|client| client.computer == shell.computer && client.pid == shell.pid)
            });

            if shell.port == forward && selected {
                removed |= self
                    .reverse_shells
                    .remove_reverse_shell(router.id, shell.computer, shell.pid)
                    .await?;
            }
        }

        Ok(Some(Ok(removed)))
    }
}

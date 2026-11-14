//! Administrator restoration of generated routers, forwards, and computers.

use greyhack_behaviour::{
    DOWNLOADS_PATH, DeviceKind, FileData, FileKind, LIBRARY_FOLDER, NetworkKind, PASSWD_PATH,
    SYSTEM_LOG_PATH, ServiceId, WEBSITE_PATH, bank_config_path, is_device, library_path,
    mail_config_path, path,
};
use greyhack_world_core::storage::nodes::NodeSpec;
use greyhack_world_missions::MissionPlacement;

use crate::generation::ComputerOptions;
use crate::generation::lan::{ComputerPlan, LanPlan};
use crate::{ComputerDraft, ComputerId, Node, Result, Router, User};

/// Administrator network restoration.
impl Restores {
    pub async fn restore_admin_network(&self, admin_computer: ComputerId) -> Result<()> {
        let Some(computer) = self.computers.computer(admin_computer).await? else {
            return Ok(());
        };
        let Some(main) = self.networks.topology.main_router(computer.network).await? else {
            return Ok(());
        };
        let Some(network) = self.networks.network(computer.network).await? else {
            return Ok(());
        };

        if self
            .networks
            .topology
            .computer(self.world.player())
            .await?
            .is_some_and(|player| player.network == network.id)
        {
            // Player-spawned networks are excluded (`Router.AdminRestoreNetwork`).
            return Ok(());
        }

        let zero_day_open = self.networks.mission_contact(network.id).await?.is_some();
        let (_, plan) = self.network_generation.lan_plan(
            &main.public_ip,
            network.kind,
            network.has_plan,
            zero_day_open,
        );
        let through_main = self.computers.computer_ids_of(main.id).await?;
        let primary = through_main.first().copied();

        for row in self.computers.computers_in(network.id).await? {
            if is_device(row.kind) || self.npcs.npcs_on(row.id).await?.is_empty() {
                continue;
            }

            let original = match &plan {
                Some(plan) => match computer_at(plan, &row.local_ip) {
                    Some(computer_plan) => Original::Plan(Box::new((
                        computer_plan.clone(),
                        plan.mission.clone(),
                        plan.kind,
                    ))),
                    None => continue,
                },
                None => Original::Draw {
                    primary: primary == Some(row.id),
                },
            };

            self.restore_npc_computer(&main, row.id, &original).await?;
        }

        // Restore every router firewall to generated rules (`ConfigBackupFirewall`,
        // `RestoreFirewall`), and its services and port table (`RouterConfigOS.RestorePorts`): only the
        // configuration page's HTTP stays, and only the main router has generated forwards.
        for router in self.networks.routers_of(network.id).await? {
            let computer = router.id.computer();

            for port in self.computers.ports(computer).await? {
                if port.service != ServiceId::Http {
                    self.computers
                        .remove_service_ports(computer, port.service)
                        .await?;
                }
            }

            if router.id != main.id {
                self.networks.clear_port_forwards(router.id).await?;
            }

            let rules = plan
                .as_ref()
                .and_then(|plan| {
                    plan.devices
                        .iter()
                        .find(|device| device.local_ip == router.local_ip)
                })
                .map(|device| device.firewall.clone())
                .unwrap_or_default();
            let mut active = router.clone();

            active.firewall_active = true;
            self.networks.save_router(&active).await?;
            self.networks.clear_firewall(router.id).await?;

            for rule in rules {
                self.networks.insert_firewall_rule(router.id, &rule).await?;
            }
        }

        // Restore main-router forwards to generated values (`RouterConfigOS.BackupPorts`,
        // `AdminRevertRouter`).
        self.networks.clear_port_forwards(main.id).await?;

        match &plan {
            Some(plan) => {
                for forward in &plan.forwards {
                    if let Some(target) = self
                        .networks
                        .topology
                        .device(network.id, &forward.lan_ip)
                        .await?
                    {
                        self.networks
                            .insert_port_forward(main.id, forward.number, target, forward.port)
                            .await?;
                    }
                }
            }
            None => {
                for computer_id in through_main {
                    for port in self.computers.ports(computer_id).await? {
                        let _ = self
                            .networks
                            .insert_port_forward(main.id, port.number, computer_id, port.number)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Restores {
    pub(crate) async fn restore_npc_computer(
        &self,
        main: &Router,
        computer_id: ComputerId,
        original: &Original,
    ) -> Result<()> {
        let Some(mut row) = self.computers.computer(computer_id).await? else {
            return Ok(());
        };
        let old_users = self.computers.users(computer_id).await?;
        let npcs = self.npcs.npcs_on(computer_id).await?;
        let mut nodes_to_retain = Vec::new();
        let mut nodes_to_retain_paths: Vec<String> = [
            ServiceId::Employees,
            ServiceId::Students,
            ServiceId::Criminals,
        ]
        .map(|service| service.config_path().to_owned())
        .into();

        nodes_to_retain_paths.push(SYSTEM_LOG_PATH.to_owned());
        for npc in &npcs {
            nodes_to_retain_paths.push(mail_config_path(&npc.user));
            nodes_to_retain_paths.push(bank_config_path(&npc.user));
        }

        for path in nodes_to_retain_paths {
            if let Some(node) = self.files.node_at(computer_id, &path).await? {
                nodes_to_retain.push(node);
            }
        }

        let kind = self
            .networks
            .network(row.network)
            .await?
            .map_or(NetworkKind::Unknown, |network| network.kind);
        let mut draft = match original {
            Original::Plan(entry) => {
                let (computer_plan, mission, planned_kind) = &**entry;

                self.network_generation.draft_of_plan(
                    &main.public_ip,
                    computer_plan,
                    *planned_kind,
                    mission.as_ref(),
                )
            }
            Original::Draw { primary } => {
                let options = ComputerOptions::for_computer(kind, *primary);

                self.network_generation
                    .generate_computer(
                        (&main.public_ip, &row.local_ip),
                        &options,
                        row.kind,
                        kind,
                        row.kind == DeviceKind::Computer,
                    )
                    .0
            }
        };

        // The regenerated computer starts with the libraries its network reported (`Router.GeneraLanPc`).
        self.updates
            .start_with_reported_libraries(row.network, &mut draft)
            .await?;

        if !self.npc_computer_needs_restore(&row, &draft, &npcs).await? {
            return Ok(());
        }

        draft.reboot_end = row.reboot_end;
        draft.active_users = npcs
            .iter()
            .filter(|npc| npc.online)
            .map(|npc| npc.user.clone())
            .collect();
        for user in &mut draft.users {
            user.person = old_users
                .iter()
                .find(|old| old.name == user.name)
                .and_then(|old| old.person);
        }

        // Retain NPC-owning user rows to avoid cascading NPC deletion; restore generated passwords
        // and identities. Remove added accounts and recreate missing original accounts.
        for user in &draft.users {
            if self
                .computers
                .user(computer_id, &user.name)
                .await?
                .is_some()
            {
                self.computers.save_user(computer_id, user).await?;
            } else {
                self.computers.insert_user(computer_id, user).await?;
            }
        }

        for old in old_users {
            if !draft.users.iter().any(|user| user.name == old.name) {
                self.computers.remove_user(computer_id, &old.name).await?;
            }
        }

        self.computers.clear_computer_contents(computer_id).await?;

        row.kind = draft.kind;
        row.name.clone_from(&draft.name);
        row.local_ip.clone_from(&draft.local_ip);
        row.next_pid = draft.next_pid;
        self.computers.save_computer(&row).await?;
        self.computers
            .insert_hardware(computer_id, &draft.hardware)
            .await?;
        for process in &draft.processes {
            self.computers.insert_process(computer_id, process).await?;
        }

        self.libraries
            .insert_full_filesystem(computer_id, &draft.filesystem)
            .await?;
        for port in &draft.ports {
            let library = self
                .files
                .library_row(port.service.library_kind(), &port.version)
                .await?;

            self.computers
                .insert_port(computer_id, port, library)
                .await?;
            self.installs
                .install_service(computer_id, port.service, &port.version)
                .await?;
        }

        if let Original::Plan(entry) = original
            && let (computer_plan, Some(mission), _) = &**entry
            && mission.assignment_at(&computer_plan.local_ip).is_some()
        {
            self.mission_placement
                .add_mission_files_over_services(
                    computer_id,
                    &main.public_ip,
                    computer_plan,
                    mission,
                    &draft,
                )
                .await?;
        }

        for user in &draft.active_users {
            self.computers.insert_session(computer_id, user).await?;
        }

        self.appliances
            .save_smart(computer_id, draft.smart.as_ref())
            .await?;
        for node in &nodes_to_retain {
            self.restore_node_copy(node).await?;
        }

        if let Some(kind) = self
            .networks
            .settings
            .network(row.network)
            .await?
            .map(|network| network.kind)
        {
            self.installs.configure_shop(computer_id, kind).await?;
        }

        Ok(())
    }

    pub async fn npc_computer_needs_restore(
        &self,
        row: &crate::Computer,
        draft: &ComputerDraft,
        npcs: &[crate::Npc],
    ) -> Result<bool> {
        if row.is_rebooting(self.world.time_seconds()) {
            return Ok(true);
        }

        let Some(lib) = self.files.node_at(row.id, LIBRARY_FOLDER).await? else {
            return Ok(true);
        };

        if self.files.children(row.id, lib.id).await?.is_empty() {
            return Ok(true);
        }

        for port in &draft.ports {
            if self
                .computers
                .service_port(row.id, port.service)
                .await?
                .is_none()
            {
                return Ok(true);
            }

            let library_path = library_path(port.service.library());

            if self
                .files
                .node_at(row.id, &library_path)
                .await?
                .is_none_or(|node| {
                    node.data()
                        .is_none_or(|data| data.kind != FileKind::Library)
                })
            {
                return Ok(true);
            }
        }

        let website = WEBSITE_PATH;

        if let Some(original) = draft
            .filesystem
            .file(website, true)
            .map(|id| draft.filesystem.node(id))
        {
            let current = self.files.node_at(row.id, website).await?;

            if current.as_ref().and_then(Node::data) != original.data() {
                return Ok(true);
            }
        }

        for folder in ["/bin", "/usr/bin", DOWNLOADS_PATH] {
            let Some(expected_folder) = draft.filesystem.folder(folder, true) else {
                continue;
            };

            for child in draft.filesystem.children(expected_folder) {
                let original = draft.filesystem.node(child);

                if !original.data().is_some_and(FileData::is_binary) {
                    continue;
                }

                let path = greyhack_behaviour::path::join(folder, &original.name);
                let current = self.files.node_at(row.id, &path).await?;

                if current.as_ref().is_none_or(|current| {
                    !current.symlink.is_empty() || current.data() != original.data()
                }) {
                    return Ok(true);
                }
            }
        }

        for npc in npcs {
            if self
                .files
                .node_at(row.id, &path::home(&npc.user))
                .await?
                .is_none_or(|node| !node.is_folder())
            {
                return Ok(true);
            }
        }

        let users = self.computers.users(row.id).await?;
        let expected_passwd = User::passwd_text(&users);
        let passwd_matches = self
            .files
            .node_at(row.id, PASSWD_PATH)
            .await?
            .and_then(|node| node.data().cloned())
            .is_some_and(|data| data.content == expected_passwd && !data.is_binary());

        Ok(!passwd_matches)
    }

    pub async fn restore_node_copy(&self, node: &Node) -> Result<()> {
        let parent_path = greyhack_behaviour::path::parent(&node.path);
        let Some(parent) = self.files.node_at(node.computer, parent_path).await? else {
            return Ok(());
        };

        if let Some(current) = self.files.node_at(node.computer, &node.path).await? {
            self.files.remove_node(node.computer, current.id).await?;
        }

        let name = node.path.rsplit('/').next().unwrap_or(&node.path);

        self.files
            .insert_node(
                node.computer,
                Some((parent.id, parent_path)),
                name,
                &NodeSpec::of(node),
            )
            .await?;

        Ok(())
    }
}

/// Original generated computer state used by administrator restoration.
pub(crate) enum Original {
    /// Computer from a planned generated LAN.
    Plan(Box<(ComputerPlan, Option<MissionPlacement>, NetworkKind)>),
    /// Computer from a fixed-size network, generated by address; the first is primary.
    Draw { primary: bool },
}

/// Returns the planned computer at `local_ip`.
fn computer_at<'p>(plan: &'p LanPlan, local_ip: &str) -> Option<&'p ComputerPlan> {
    plan.devices
        .iter()
        .flat_map(|device| &device.computers)
        .find(|computer| computer.local_ip == local_ip)
}

/// A world service with only its declared dependencies.
pub struct Restores {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) mission_placement: greyhack_world_core::Dep<crate::MissionPlacement>,
    pub(crate) network_generation: greyhack_world_core::Dep<crate::NetworkGeneration>,
    pub(crate) updates: greyhack_world_core::Dep<greyhack_world_libraries::Updates>,
    pub(crate) installs: greyhack_world_core::Dep<greyhack_world_services::Installs>,
    pub(crate) libraries: greyhack_world_core::Dep<greyhack_world_libraries::LibraryService>,
    pub(crate) npcs: greyhack_world_core::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) appliances: greyhack_world_core::Dep<greyhack_world_smart_devices::Appliances>,
}

impl Restores {
    pub(crate) fn new(registry: &std::rc::Rc<greyhack_world_modules::Registry>) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            mission_placement: registry.get(),
            network_generation: registry.get(),
            updates: registry.get(),
            installs: registry.get(),
            libraries: registry.get(),
            npcs: registry.get(),
            appliances: registry.get(),
        }
    }
}

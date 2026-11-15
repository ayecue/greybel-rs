//! The world changes of a procedural mission: its computers' accounts, files, logs, mails and the passwords it
//! teaches (`NetworkLan.CreateMission`, `Router.PostGenMission`, `Router.PrepareProceduralObjective`).

use greyhack_behaviour::{
    FileData, LogEntry, NetworkKind, PermissionSet, ROOT_USER, ROUTER_NODE_LABEL,
    SYSTEM_LOG_FOLDER, SYSTEM_LOG_NAME, ServiceId, SystemLog, kind_for_ip, path,
};
use greyhack_world_assets::{GameTextTable, website_default};
use greyhack_world_missions::{
    Assignment, ComputerContent, ComputerMission, Context, MissionFile,
    MissionPlacement as MissionPlan, PreGenNpc, computer_content, router_log, seed_from_ip,
};

use super::mission_drafts::{
    add_content, ensure_folder, mission_node, rename_account, replaced_account, system_log_data,
};
use crate::generation::lan::{ComputerPlan, LanPlan};
use crate::{ComputerDraft, ComputerId, NetworkId, NodeDraft, Result, Word};

/// Procedural-mission placement.
impl MissionPlacement {
    pub fn apply_mission_to_draft(
        &self,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        placement: &MissionPlan,
        draft: &mut ComputerDraft,
    ) {
        let Some(assignment) = placement.assignment_at(&computer_plan.local_ip) else {
            return;
        };
        let mission = &placement.request.part.computers[assignment.computer];

        // `Computer.ConfiguringServices` writes the website of an http service before `PostGenMission` adds the
        // mission's files, so a mission's web file replaces it. The website of a computer the mission gave the http
        // service is the text of the item (`NetworkLan.ConfigureChainMission`, `LanPcInfo.website`).
        if computer_plan
            .services
            .iter()
            .any(|plan| plan.service == ServiceId::Http)
        {
            let website = match placement
                .websites
                .iter()
                .find(|(local_ip, _)| *local_ip == computer_plan.local_ip)
            {
                Some((_, text_id)) if text_id.is_empty() => String::new(),
                Some((_, text_id)) => GameTextTable::english()
                    .get(text_id)
                    .unwrap_or(text_id.as_str())
                    .to_owned(),
                None => website_default().to_owned(),
            };
            let folder = ensure_folder(&mut draft.filesystem, "/Public/htdocs", ROOT_USER);

            draft.filesystem.add(
                folder,
                NodeDraft::file(
                    "website.html",
                    ROOT_USER,
                    PermissionSet::default_for(false),
                    FileData::text(&website),
                ),
            );
        }

        if let Some(npc) = usize::try_from(mission.clone_npc_pc_idx)
            .ok()
            .and_then(|index| placement.npcs().get(index))
            && let Some(old) = replaced_account(draft, npc, assignment)
        {
            rename_account(draft, &old, &npc.surname, npc.forced_pass.as_deref());
        }

        let Some(content) =
            self.mission_content(public_ip, &computer_plan.local_ip, placement, draft)
        else {
            return;
        };

        add_content(&mut draft.filesystem, &content);
    }

    pub async fn add_mission_files_over_services(
        &self,
        computer: ComputerId,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        placement: &MissionPlan,
        draft: &ComputerDraft,
    ) -> Result<()> {
        let Some(content) =
            self.mission_content(public_ip, &computer_plan.local_ip, placement, draft)
        else {
            return Ok(());
        };

        for file in &content.files {
            let Some(folder) = self
                .files
                .lookup
                .folder(computer, &file.folder, false)
                .await?
            else {
                continue;
            };
            let node = mission_node(file);
            let path = path::join(&file.folder, &file.name);

            if let Some(current) = self.files.lookup.file(computer, &path, false).await? {
                if current.owner == node.owner
                    && current.permissions == node.permissions
                    && current.data() == node.data()
                {
                    continue;
                }

                self.files.changes.remove_node(&current).await?;
            }

            self.generator.add_full_node(&folder, &node).await?;
        }

        Ok(())
    }

    pub fn mission_content_of(
        &self,
        public_ip: &str,
        local_ip: &str,
        placement: &MissionPlan,
        users: &[(String, String)],
        computer: usize,
    ) -> Option<ComputerContent> {
        let group = &placement.request.group;
        let remote = |network: usize| -> Vec<String> {
            let Some(address) = group.get(network) else {
                return Vec::new();
            };
            let kind = kind_for_ip(&self.world.world_seed(), address);

            self.network_generation
                .lan_plan(address, kind, true, true)
                .1
                .and_then(|plan| plan.mission)
                .map(|placement| placement.local_ips)
                .unwrap_or_default()
        };
        let network_seed = self
            .world
            .world_seed()
            .derive_int(ROUTER_NODE_LABEL, &[public_ip]);
        let context = Context {
            public_ip,
            group,
            local_ips: &placement.local_ips,
            npcs: placement.npcs(),
            remote_local_ips: &remote,
            users,
            now: self.world.time_seconds(),
            zero_day_ip: "",
            seed: network_seed.wrapping_add(seed_from_ip(local_ip)),
            objective_seed: self
                .world
                .world_seed()
                .derive_int("procedural-objective", &[public_ip, local_ip]),
        };

        computer_content(&context, &placement.request.part.computers[computer])
    }

    pub fn mission_router_log(&self, placement: &MissionPlan) -> Vec<LogEntry> {
        let computers: Vec<(String, &ComputerMission)> = placement
            .assignments
            .iter()
            .map(|assignment| {
                (
                    assignment.local_ip.clone(),
                    &placement.request.part.computers[assignment.computer],
                )
            })
            .collect();

        router_log(
            &placement.request.group,
            &computers,
            self.world.time_seconds(),
        )
    }

    pub fn add_router_log(&self, draft: &mut ComputerDraft, entries: Vec<LogEntry>) {
        if entries.is_empty() {
            return;
        }

        let log = SystemLog {
            content_log: entries,
        };

        add_content(
            &mut draft.filesystem,
            &ComputerContent {
                owner: ROOT_USER.to_owned(),
                files: vec![MissionFile {
                    folder: SYSTEM_LOG_FOLDER.to_owned(),
                    name: SYSTEM_LOG_NAME.to_owned(),
                    data: system_log_data(&log),
                    owner: ROOT_USER.to_owned(),
                    permissions: None,
                    symlink: String::new(),
                    timestamp: None,
                    folder_permissions: None,
                    create_folder: true,
                }],
                ..ComputerContent::default()
            },
        );
    }

    pub async fn finish_mission(
        &self,
        network: NetworkId,
        public_ip: &str,
        plan: &LanPlan,
        placement: &MissionPlan,
    ) -> Result<()> {
        // `NetworkLan.missionCreated`; the starter mission mail names the group's first network, or the first mission
        // computer when that network is this one (`Persona.GenerateMailStarterMission`).
        if let Some(first_local) = placement.local_ips.first() {
            let contact = match placement.request.group.first() {
                Some(first) if first != public_ip => first,
                _ => first_local,
            };

            self.networks.set_mission_contact(network, contact).await?;
        }

        for assignment in &placement.assignments {
            let Some(computer) = self
                .networks
                .topology
                .device(network, &assignment.local_ip)
                .await?
            else {
                continue;
            };
            let mission = &placement.request.part.computers[assignment.computer];
            let mut users: Vec<(String, String)> = self
                .computers
                .users(computer)
                .await?
                .into_iter()
                .map(|user| (user.name, user.password))
                .collect();

            users.sort_by_key(|(name, _)| name != ROOT_USER);

            let Some(content) = self.mission_content_of(
                public_ip,
                &assignment.local_ip,
                placement,
                &users,
                assignment.computer,
            ) else {
                continue;
            };

            if let Some(npc) = usize::try_from(mission.clone_npc_pc_idx)
                .ok()
                .and_then(|index| placement.npcs().get(index))
            {
                let alias = self.mission_mail_alias(public_ip, plan.kind, plan, assignment, npc);

                self.ensure_mission_npc(
                    computer,
                    &npc.surname,
                    &npc.complete_name(),
                    alias.as_deref(),
                )
                .await?;
            }

            for password in &content.passwords {
                self.computers.users.register_password(password).await?;
            }

            if content.mails.is_empty() {
                continue;
            }

            let Some(person) = self
                .ensure_mission_npc(computer, &content.owner, "", None)
                .await?
            else {
                continue;
            };
            let Some(address) = self.mail.mail_address(person).await? else {
                continue;
            };

            for mail in &content.mails {
                let (send, receive) = if mail.outsider_is_sender {
                    (mail.outsider.as_str(), address.as_str())
                } else {
                    (address.as_str(), mail.outsider.as_str())
                };

                self.add_mission_mail(&address, &mail.title, send, receive, &mail.lines)
                    .await?;
            }
        }

        Ok(())
    }

    pub fn takes_missions(kind: NetworkKind) -> bool {
        kind != NetworkKind::RentServer
    }
}

impl MissionPlacement {
    pub fn mission_content(
        &self,
        public_ip: &str,
        local_ip: &str,
        placement: &MissionPlan,
        draft: &ComputerDraft,
    ) -> Option<ComputerContent> {
        let assignment = placement.assignment_at(local_ip)?;
        let users: Vec<(String, String)> = draft
            .users
            .iter()
            .map(|user| (user.name.clone(), user.password.clone()))
            .collect();

        self.mission_content_of(public_ip, local_ip, placement, &users, assignment.computer)
    }

    pub fn mission_mail_alias(
        &self,
        public_ip: &str,
        kind: NetworkKind,
        plan: &LanPlan,
        assignment: &Assignment,
        npc: &PreGenNpc,
    ) -> Option<String> {
        if !assignment.npc_in_lan {
            return None;
        }

        let computer_plan = plan
            .devices
            .iter()
            .flat_map(|device| &device.computers)
            .find(|computer| computer.local_ip == assignment.local_ip)?;
        let draft = self
            .network_generation
            .draft_of_plan(public_ip, computer_plan, kind, None);

        replaced_account(&draft, npc, assignment).filter(|old| *old != npc.surname)
    }

    pub async fn ensure_mission_npc(
        &self,
        computer: ComputerId,
        user: &str,
        name: &str,
        mail_alias: Option<&str>,
    ) -> Result<Option<crate::PersonId>> {
        if let Some(npc) = self
            .npcs
            .npcs_on(computer)
            .await?
            .into_iter()
            .find(|npc| npc.user == user)
        {
            if let Some(mut person) = self.computers.person(npc.person).await?
                && !name.trim().is_empty()
                && person.name != name
            {
                name.clone_into(&mut person.name);
                self.computers.save_person(&person).await?;
            }

            if let Some(alias) = mail_alias {
                let domain = self.mail.mail_domain(computer).await?;

                self.npcs
                    .set_person_mail_address(npc.person, &format!("{alias}@{domain}"))
                    .await?;
            }

            return Ok(Some(npc.person));
        }

        let mut rng = self.world.world_mut().rng.fork("mission-npc");
        let name = if name.trim().is_empty() {
            format!("{} {user}", Word::Surname.generate(&mut rng))
        } else {
            name.to_owned()
        };
        let next = self.world.time_seconds();

        self.npcs
            .add_npc_with_mail_name(computer, user, mail_alias.unwrap_or(user), &name, next)
            .await
    }

    pub async fn add_mission_mail(
        &self,
        address: &str,
        title: &str,
        send: &str,
        receive: &str,
        lines: &[(u8, String)],
    ) -> Result<()> {
        let mut id: Option<String> = None;

        for (speaker, message) in lines {
            let from = if *speaker == 0 { send } else { receive };
            let body = if from == send {
                format!("> {message}")
            } else {
                message.clone()
            };

            match &id {
                None => {
                    id = Some(self.mail.insert_email(address, from, title, &body).await?);
                }
                Some(id) => {
                    self.mail.insert_reply(address, id, &body).await?;
                }
            }
        }

        Ok(())
    }
}

/// A world service with only its declared dependencies.
pub struct MissionPlacement {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) generator: greyhack_world_core::Dep<greyhack_world_libraries::Generator>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    pub(crate) npcs: greyhack_world_core::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) network_generation: greyhack_world_core::Dep<crate::NetworkGeneration>,
}

impl MissionPlacement {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            generator: registry.get(),
            mail: registry.get(),
            npcs: registry.get(),
            network_generation: registry.get(),
        }
    }
}

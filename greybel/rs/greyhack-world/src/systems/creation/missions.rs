//! The world changes of a procedural mission: its computers' accounts, files, logs, mails and the passwords it
//! teaches (`NetworkLan.CreateMission`, `Router.PostGenMission`, `Router.PrepareProceduralObjective`).

use greyhack_behaviour::{
    LogEntry, NetworkKind, ROOT_USER, ROUTER_NODE_LABEL, SYSTEM_LOG_FOLDER, SYSTEM_LOG_NAME,
    SystemLog, kind_for_ip,
};

use super::mission_drafts::{add_content, rename_account, replaced_account, system_log_data};
use crate::generation::lan::{ComputerPlan, LanPlan};
use crate::generation::missions::{
    Assignment, ComputerContent, Context, MissionFile, MissionPlacement, PreGenNpc,
    computer_content, router_log, seed_from_ip,
};
use crate::{ComputerDraft, ComputerId, NetworkId, Result, Word, World};

impl World {
    /// `Router.GeneraLanPc`, `PostGenMission`: gives the draft of a mission computer its accounts, files and log.
    /// The mails and passwords of the mission need the world (see [`Self::finish_mission`]).
    pub(super) fn apply_mission_to_draft(
        &self,
        public_ip: &str,
        computer_plan: &ComputerPlan,
        placement: &MissionPlacement,
        draft: &mut ComputerDraft,
    ) {
        let Some(assignment) = placement.assignment_at(&computer_plan.local_ip) else {
            return;
        };
        let mission = &placement.request.part.computers[assignment.computer];

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

    /// The content `PostGenMission` gives the computer at `local_ip` of a mission network with `users`.
    fn mission_content(
        &self,
        public_ip: &str,
        local_ip: &str,
        placement: &MissionPlacement,
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

    pub(super) fn mission_content_of(
        &self,
        public_ip: &str,
        local_ip: &str,
        placement: &MissionPlacement,
        users: &[(String, String)],
        computer: usize,
    ) -> Option<ComputerContent> {
        let group = &placement.request.group;
        let remote = |network: usize| -> Vec<String> {
            let Some(address) = group.get(network) else {
                return Vec::new();
            };
            let kind = kind_for_ip(&self.world_seed, address);

            self.lan_plan(address, kind, true)
                .1
                .and_then(|plan| plan.mission)
                .map(|placement| placement.local_ips)
                .unwrap_or_default()
        };
        let network_seed = self.world_seed.derive_int(ROUTER_NODE_LABEL, &[public_ip]);
        let context = Context {
            public_ip,
            group,
            local_ips: &placement.local_ips,
            npcs: placement.npcs(),
            remote_local_ips: &remote,
            users,
            now: self.time_seconds,
            seed: network_seed.wrapping_add(seed_from_ip(local_ip)),
            objective_seed: self
                .world_seed
                .derive_int("procedural-objective", &[public_ip, local_ip]),
        };

        computer_content(&context, &placement.request.part.computers[computer])
    }

    /// The bounce entries the main router of the network contains (`NetworkLan.ConfigureRouterLogs`).
    pub(super) fn mission_router_log(&self, placement: &MissionPlacement) -> Vec<LogEntry> {
        let computers: Vec<(String, &crate::generation::missions::ComputerMission)> = placement
            .assignments
            .iter()
            .map(|assignment| {
                (
                    assignment.local_ip.clone(),
                    &placement.request.part.computers[assignment.computer],
                )
            })
            .collect();

        router_log(&placement.request.group, &computers, self.time_seconds)
    }

    /// Gives the main router its log (`NetworkLan.ConfigureRouterLogs`).
    pub(super) fn add_router_log(&self, draft: &mut ComputerDraft, entries: Vec<LogEntry>) {
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

    /// The part of `PostGenMission` that needs the NPCs of the network: the mission NPCs themselves, the mailboxes
    /// the mails go to, and the passwords the decipher database learns.
    pub(super) async fn finish_mission(
        &mut self,
        network: NetworkId,
        public_ip: &str,
        kind: NetworkKind,
        plan: &LanPlan,
        placement: &MissionPlacement,
    ) -> Result<()> {
        // `NetworkLan.missionCreated`; the starter mission mail names the group's first network, or the first mission
        // computer when that network is this one (`Persona.GenerateMailStarterMission`).
        if let Some(first_local) = placement.local_ips.first() {
            let contact = match placement.request.group.first() {
                Some(first) if first != public_ip => first,
                _ => first_local,
            };

            self.db.set_mission_contact(network, contact).await?;
        }

        for assignment in &placement.assignments {
            let Some(computer) = self.device(network, &assignment.local_ip).await? else {
                continue;
            };
            let mission = &placement.request.part.computers[assignment.computer];
            let mut users: Vec<(String, String)> = self
                .db
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
                let alias = self.mission_mail_alias(public_ip, kind, plan, assignment, npc);

                self.ensure_mission_npc(
                    computer,
                    &npc.surname,
                    &npc.complete_name(),
                    alias.as_deref(),
                )
                .await?;
            }

            for password in &content.passwords {
                self.register_password(password).await?;
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
            let Some(address) = self
                .db
                .person(person)
                .await?
                .and_then(|person| person.mail_address)
            else {
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

    /// The name of the mail address of the mission NPC `npc` on the computer of `assignment`, when the account is
    /// not named like it: an NPC that is already on another computer of the network takes over the account of the NPC
    /// it replaces and has that NPC's address (`NetworkLan.UpdateNpcMissionNames`, `NpcInfo.UpdateNpcInfo`'s
    /// `mailUserAlias`).
    fn mission_mail_alias(
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
        let draft = self.draft_of_plan(public_ip, computer_plan, kind, None);

        replaced_account(&draft, npc, assignment).filter(|old| *old != npc.surname)
    }

    /// Gives the account `user` of `computer` an NPC named `name`, or renames the NPC it has (`NpcInfo.UpdateNpcInfo`).
    /// The NPC's mail address is `mail_alias@domain` when there is an alias.
    async fn ensure_mission_npc(
        &mut self,
        computer: ComputerId,
        user: &str,
        name: &str,
        mail_alias: Option<&str>,
    ) -> Result<Option<crate::PersonId>> {
        if let Some(npc) = self
            .db
            .npcs_on(computer)
            .await?
            .into_iter()
            .find(|npc| npc.user == user)
        {
            if let Some(mut person) = self.db.person(npc.person).await?
                && !name.trim().is_empty()
                && person.name != name
            {
                name.clone_into(&mut person.name);
                self.db.save_person(&person).await?;
            }

            if let Some(alias) = mail_alias {
                let domain = self.mail_domain(computer).await?;

                self.set_person_mail_address(npc.person, &format!("{alias}@{domain}"))
                    .await?;
            }

            return Ok(Some(npc.person));
        }

        let mut rng = self.rng.fork("mission-npc");
        let name = if name.trim().is_empty() {
            format!("{} {user}", Word::Surname.generate(&mut rng))
        } else {
            name.to_owned()
        };
        let next = self.time_seconds;

        self.add_npc_with_mail_name(computer, user, mail_alias.unwrap_or(user), &name, next)
            .await
    }

    /// `ItemsWorldGen.AddNpcConvMail`: puts a conversation in a mailbox as one mail whose first message is `lines[0]`;
    /// the speaker that is `send` has its lines marked as sent.
    async fn add_mission_mail(
        &mut self,
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
                    id = Some(self.db.insert_email(address, from, title, &body).await?);
                }
                Some(id) => {
                    self.db.insert_reply(address, id, &body).await?;
                }
            }
        }

        Ok(())
    }

    /// The network kinds a mission cannot be put in (`ServerMap.GeneraRouter`).
    pub(super) fn takes_missions(kind: NetworkKind) -> bool {
        kind != NetworkKind::RentServer
    }
}

//! Generated network NPCs, smartphones, and administrator selection.

use greyhack_behaviour::{DeviceKind, NPCS_PER_NETWORK, NetworkKind, ROOT_USER, admin_candidates};

use crate::generation::{ComputerOptions, computer};
use crate::{
    ComputerId, Gender, JobRole, LanPlace, NetworkAdmin, PersonId, Result, Rng, RouterId, World,
};

impl World {
    /// Adds 1–3 non-root NPC users to `computers` using network `rng` after other generation.
    /// Initializes persistent work schedules immediately and schedules work-block transitions
    /// (`NPCSchedule.NPCSchedule`, `Persona.IsLogged`). Adds smartphones through
    /// `NetworkLan.GenerateSmartPhones`.
    pub(super) async fn add_network_npcs(
        &mut self,
        computers: &[ComputerId],
        rng: &mut Rng,
    ) -> Result<()> {
        if computers.is_empty() {
            return Ok(());
        }

        // One NPC per account (`npcs.user_id` unique). A repeated pick of an account still draws
        // like a new NPC before it is skipped, so the draws that follow do not depend on repeats.
        let mut npc_users = std::collections::HashSet::new();
        let mut npcs = Vec::new();

        for _ in 0..rng.range(*NPCS_PER_NETWORK.start(), *NPCS_PER_NETWORK.end()) {
            let computer = *rng.pick(computers);
            let users: Vec<String> = self
                .db
                .users(computer)
                .await?
                .into_iter()
                .map(|user| user.name)
                .filter(|user| user != ROOT_USER)
                .collect();

            if users.is_empty() {
                continue;
            }

            let user = rng.pick(&users).clone();
            // NPC surnames match account names (`NetworkLan.AddNpcUser`).
            let name = format!("{} {user}", Gender::pick(rng).first_name(rng));

            // The network generator draws once more per NPC, so that what is generated after the
            // NPCs does not depend on how their first switch is timed.
            rng.skip(1);

            let next = self.time_seconds;

            if !npc_users.insert((computer, user.clone())) {
                continue;
            }

            if let Some(npc) = self.add_npc(computer, &user, &name, next).await? {
                npcs.push((npc, computer, user.clone()));
                self.add_npc_phone(computer, npc, &user, rng).await?;
            }
        }

        let Some(network) = self
            .db
            .computer(computers[0])
            .await?
            .map(|computer| computer.network)
        else {
            return Ok(());
        };

        self.assign_jobs(network, &npcs).await?;
        self.select_admin(&npcs).await?;

        self.select_traffic_credentials(network).await
    }

    /// Selects a network administrator from newly added `npcs` (`NetworkLan.SelectAdmin`),
    /// preferring computers with multiple NPCs. Uses standard NPC job stats (`NpcInfo`) and an
    /// isolated generator to preserve other content. Rented-server networks have none. Grey Hack
    /// always provides a multi-NPC computer; sparse generated networks may select any NPC to ensure
    /// an administrator can restore the network.
    async fn select_admin(&mut self, npcs: &[(PersonId, ComputerId, String)]) -> Result<()> {
        let Some((_, first, _)) = npcs.first() else {
            return Ok(());
        };
        let Some(row) = self.db.computer(*first).await? else {
            return Ok(());
        };
        let Some(network) = self.db.network(row.network).await? else {
            return Ok(());
        };

        if network.kind == NetworkKind::RentServer {
            return Ok(());
        }

        let computers: Vec<ComputerId> = npcs.iter().map(|(_, computer, _)| *computer).collect();
        let candidates: Vec<&(PersonId, ComputerId, String)> = admin_candidates(&computers)
            .into_iter()
            .map(|index| &npcs[index])
            .collect();

        let public_ip = self.public_ip(*first).await?;
        let mut rng = Rng::stable(self.seed, &[&public_ip, "admin"]);
        let (person, _, _) = *rng.pick(&candidates);
        let admin = NetworkAdmin::draw(&mut rng);

        // Set the administrator role to admin (`NpcInfo.UpdateJobRole`).
        if let Some(mut npc) = self.db.npc(*person).await? {
            npc.job_role = JobRole::Admin;
            npc.work_performance = admin.work_performance;
            self.db.save_npc_job(&npc).await?;
        }

        self.db
            .set_network_admin(row.network, Some(*person), &admin)
            .await
    }

    /// Adds `npc`'s smartphone for `owner` (`NetworkLan.GenerateSmartPhones`). Connects to
    /// `owner`'s gateway Wi-Fi when available, otherwise main-router Wi-Fi, using the next subnet
    /// address.
    pub(in crate::systems) async fn add_phone(
        &mut self,
        owner: ComputerId,
        npc: PersonId,
        options: &ComputerOptions,
    ) -> Result<Option<ComputerId>> {
        let Some(owner) = self.db.computer(owner).await? else {
            return Ok(None);
        };
        let Some(lan) = self.lan(owner.network).await? else {
            return Ok(None);
        };
        let Some(main) = self.main_router(owner.network).await? else {
            return Ok(None);
        };
        let gateway = match lan.find(&owner.gateway) {
            Some(node) if lan.node(node).kind == DeviceKind::Router => {
                self.db.router(RouterId::of(lan.node(node).key)).await?
            }
            _ => None,
        };
        let device = gateway
            .filter(|router| router.essid.is_some())
            .unwrap_or_else(|| main.clone());
        let address = lan.next_address(&device.local_ip);
        let mut rng = Rng::stable(self.seed, &[&main.public_ip, &address]);
        let (mut draft, _) =
            computer::computer(&mut rng, &address, options, DeviceKind::Smartphone);

        draft.npc_owner = Some(npc);
        self.start_with_reported_libraries(owner.network, &mut draft)
            .await?;

        // Generated computers connect through their network main router (`Router.GeneraLanPc`).
        let place = LanPlace::computer_under(device.id.computer(), main.id, &device.local_ip);
        let phone = self
            .db
            .insert_computer(&draft, owner.network, &place)
            .await?;

        self.add_clock_program(phone).await?;

        Ok(Some(phone))
    }
}

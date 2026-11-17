//! Zero-day rumor networks and deterministic procedural mission fixtures.

use greyhack_behaviour::{
    CompanyAlign, FileData, FileKind, GROUP_COUNT, GUEST_USER, Gender, LibraryKind, MissionKind,
    NetworkKind, PermissionSet, ROOT_USER, Reward, Rng, is_rumor_time, random_group_for_mission,
};
use greyhack_world_assets::GameTextTable;
use greyhack_world_core::{ComputerId, NetworkId, Result, RouterId};
use greyhack_world_libraries::{Generator, Library, MemoryZone, NodeDraft, Weakness};
use greyhack_world_npcs::Npc;

use crate::NetworkOptions;

/// The first-rumor network and its delivered `EMAIL_FIRST_RUMOR` mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RumorMission {
    pub public_ip: String,
    pub lan_ip: String,
    pub sender: String,
    pub body: String,
}

/// The result of the mission window's `GetRumorServerRpc` action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RumorRequest {
    Rumor(RumorMission),
    Failure(String),
}

/// Creates zero-day networks and procedural missions through the shared network generator.
pub struct ZeroDayMissions {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) network_generation: greyhack_world_modules::Dep<crate::NetworkGeneration>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) generator: greyhack_world_modules::Dep<Generator>,
    pub(crate) debug: greyhack_world_modules::Dep<greyhack_world_libraries::services::Debug>,
    pub(crate) jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
    pub(crate) mail: greyhack_world_modules::Dep<greyhack_world_npcs::Mail>,
    pub(crate) npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
}

impl ZeroDayMissions {
    #[expect(
        clippy::too_many_arguments,
        reason = "The service receives its independent repositories and domain dependencies from the module root"
    )]
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        network_generation: greyhack_world_modules::Dep<crate::NetworkGeneration>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        generator: greyhack_world_modules::Dep<Generator>,
        debug: greyhack_world_modules::Dep<greyhack_world_libraries::services::Debug>,
        jobs: greyhack_world_modules::Dep<greyhack_world_npcs::JobService>,
        mail: greyhack_world_modules::Dep<greyhack_world_npcs::Mail>,
        npcs: greyhack_world_modules::Dep<greyhack_world_npcs::NpcService>,
    ) -> Self {
        Self {
            world: world.clone(),
            network_generation,
            computers,
            files,
            generator,
            debug,
            jobs,
            mail,
            npcs,
        }
    }

    /// Runs `IsAvailableRumor`'s lazy network lifecycle before its per-player request check.
    pub async fn is_available_rumor(&self, player_id: &str, sync_db: bool) -> Result<bool> {
        let open = is_rumor_time(self.world.time_seconds());
        let ip = self.debug.rumor_ip().await?;

        if !open && !ip.is_empty() {
            if let Some(router) = self
                .network_generation
                .networks
                .routers
                .router_at(&ip)
                .await?
            {
                self.network_generation
                    .networks
                    .remove_network(router.network)
                    .await?;
            }
            self.debug.clear_rumor().await?;
        } else if open && ip.is_empty() {
            self.generate_network_final_rumor().await?;
        }

        if !open {
            return Ok(false);
        }

        self.debug.is_available_rumor(player_id, sync_db).await
    }

    /// Returns whether visiting a HackShop offers zero-day access (`AccessWebServerRpc`).
    pub async fn hack_shop_rumor_available(&self, public_ip: &str) -> Result<bool> {
        let Some(router) = self
            .network_generation
            .networks
            .routers
            .router_at(public_ip)
            .await?
        else {
            return Ok(false);
        };
        let Some(network) = self
            .network_generation
            .networks
            .network(router.network)
            .await?
        else {
            return Ok(false);
        };
        if network.kind != NetworkKind::HackShop {
            return Ok(false);
        }

        self.is_available_rumor(&self.debug.player_id(), false)
            .await
    }

    /// Requests the first-rumor mission and mails it exactly where `GetRumorServerRpc` does.
    pub async fn request_zero_day_mission(&self) -> Result<RumorRequest> {
        let Some(address) = self.mail.mail_address(self.world.player_person()).await? else {
            return Ok(RumorRequest::Failure(
                "You need to have an email account to continue.".to_owned(),
            ));
        };
        if !self
            .is_available_rumor(&self.debug.player_id(), true)
            .await?
        {
            return Ok(RumorRequest::Failure(
                "There are no rumors at the moment.".to_owned(),
            ));
        }

        let (public_ip, lan_ip) = self.generate_network_first_rumor().await?;
        let body = game_text("EMAIL_FIRST_RUMOR")
            .replace("$LOCAL_IP", &lan_ip)
            .replace("$PUBLIC_IP", &public_ip);
        let sender = self.network_sender(&public_ip).await?;
        self.mail
            .insert_email(
                &address,
                &sender,
                "It seems that there are some rumors.",
                &body,
            )
            .await?;

        Ok(RumorRequest::Rumor(RumorMission {
            public_ip,
            lan_ip,
            sender,
            body,
        }))
    }

    /// Spawns Comisaria/Universidades/MobileShop and marks one NPC with `ConfigRumors`.
    pub async fn generate_network_first_rumor(&self) -> Result<(String, String)> {
        let mut random = self.world.world_mut().rng.fork("zero-day-first-rumor");
        let kind = *random.pick(&[
            NetworkKind::Police,
            NetworkKind::University,
            NetworkKind::MobileShop,
        ]);
        let router = self
            .network_generation
            .add_network(&NetworkOptions {
                kind: Some(kind),
                allow_missions: false,
                ..NetworkOptions::default()
            })
            .await?;
        let public_ip = self.public_ip(router).await?;
        let lan_ip = self.configure_rumors(router, &mut random).await?;

        Ok((public_ip, lan_ip))
    }

    /// Spawns the Neurobox final network, configures its NPCs, then saves its public address.
    async fn generate_network_final_rumor(&self) -> Result<String> {
        let router = self
            .network_generation
            .add_network(&NetworkOptions {
                kind: Some(NetworkKind::Neurobox),
                allow_missions: true,
                ..NetworkOptions::default()
            })
            .await?;
        let public_ip = self.public_ip(router).await?;
        self.configure_zero_day_info(router, &public_ip).await?;
        self.debug.set_rumor_ip(&public_ip).await?;

        Ok(public_ip)
    }

    async fn configure_rumors(&self, router: RouterId, random: &mut Rng) -> Result<String> {
        let network = self.network(router).await?;
        let npcs = self.npcs.npcs_in(network).await?;
        if npcs.is_empty() {
            return Ok(String::new());
        }
        let npc = random.pick(&npcs).clone();

        self.jobs.set_npc_rumor(npc.person).await?;
        let final_ip = self.debug.rumor_ip().await?;
        let final_npc = self.rumor_npc_name().await?;
        self.write_rumor_chat(&npc, &final_ip, &final_npc, random)
            .await?;

        let lan_ip = self
            .computers
            .computer(npc.computer)
            .await?
            .map_or_else(String::new, |computer| computer.local_ip);

        Ok(lan_ip)
    }

    /// Marks one to three NPCs with company alignment and a distinct day-zero contact.
    async fn configure_zero_day_info(&self, router: RouterId, public_ip: &str) -> Result<()> {
        let network = self.network(router).await?;
        let computers = self
            .computers
            .computers_in(network)
            .await?
            .into_iter()
            .map(|computer| computer.id)
            .collect::<Vec<_>>();
        let mut random = self.world.world_mut().rng.fork("zero-day-network-info");

        while self.npcs.npcs_in(network).await?.len() < 2 {
            self.network_generation
                .add_network_npcs(&computers, &mut random)
                .await?;
        }

        let mut remaining = self.npcs.npcs_in(network).await?;
        let count = usize::try_from(random.range(1, 3))
            .unwrap_or(1)
            .min(remaining.len() - 1);

        for index in 0..count {
            let selected = usize::try_from(
                random.range(0, u32::try_from(remaining.len() - 1).unwrap_or(u32::MAX)),
            )
            .unwrap_or(0);
            let npc = remaining.remove(selected);
            let contact = random.pick(&remaining).clone();
            let contact_mail = self
                .mail
                .mail_address(contact.person)
                .await?
                .unwrap_or_default();
            let alignment = u8::try_from(random.range(1, 2))
                .ok()
                .and_then(CompanyAlign::from_code)
                .expect("the random alignment is valid");
            self.jobs
                .set_npc_company_align(
                    npc.person,
                    alignment,
                    Some(contact_mail),
                    u32::try_from(index).ok(),
                )
                .await?;
        }

        // A final network has at least one aligned NPC after the rule above; derive this only after
        // saving so first-rumor chats resolve the same name as NetworkLan.GetNpcRumorName.
        debug_assert!(self.rumor_npc_name_at(public_ip).await?.is_some());
        Ok(())
    }

    async fn rumor_npc_name(&self) -> Result<String> {
        let ip = self.debug.rumor_ip().await?;

        self.rumor_npc_name_at(&ip)
            .await
            .map(Option::unwrap_or_default)
    }

    async fn rumor_npc_name_at(&self, public_ip: &str) -> Result<Option<String>> {
        let Some(router) = self
            .network_generation
            .networks
            .routers
            .router_at(public_ip)
            .await?
        else {
            return Ok(None);
        };

        Ok(self
            .npcs
            .npcs_in(router.network)
            .await?
            .into_iter()
            .filter(|npc| npc.zero_day_index.is_some())
            .min_by_key(|npc| npc.zero_day_index)
            .map(|npc| npc.user))
    }

    async fn write_rumor_chat(
        &self,
        npc: &Npc,
        final_ip: &str,
        final_npc: &str,
        random: &mut Rng,
    ) -> Result<()> {
        let obsolete = final_ip.is_empty();
        let conversation = if obsolete {
            "CONV_RUMOR_OBSOLETE".to_owned()
        } else {
            format!("CONV_RUMOR_{}", random.range(1, 6))
        };
        let receive = Gender::pick(random).first_name(random);
        let name = format!("chat{}.log", random.range(0, 999));
        let mut public_ips = Vec::new();
        let mut npc_indices = Vec::new();

        if !obsolete {
            for key in conversation_tags(&conversation, "$IP_IDX") {
                public_ips.push((key, hide_ip(final_ip, random)));
            }
            for key in conversation_tags(&conversation, "$NPC_IDX") {
                npc_indices.push((key, final_npc.to_owned()));
            }
        }

        let map = |entries: Vec<(String, String)>| {
            entries
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(&key).expect("a JSON string"),
                        serde_json::to_string(&value).expect("a JSON string")
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        };
        let content = format!(
            r#"{{"chatID":{},"users":{{"SEND":{},"RECEIVE":{}}},"idxPublicIPs":{{{}}},"idxLocalIPs":{{}},"idxNpcs":{{{}}},"runtimeTags":{{}},"zeroDayNetworkIP":""}}"#,
            serde_json::to_string(&conversation).expect("a JSON string"),
            serde_json::to_string(&npc.user).expect("a JSON string"),
            serde_json::to_string(&receive).expect("a JSON string"),
            map(public_ips),
            map(npc_indices),
        );
        self.files
            .changes
            .write_file_as(
                npc.computer,
                &format!("/home/{}/{}", npc.user, name),
                &npc.user,
                FileData {
                    kind: FileKind::ChatLog,
                    size: FileData::binary_size(&name, false),
                    content,
                    ..FileData::default()
                },
            )
            .await
    }

    async fn public_ip(&self, router: RouterId) -> Result<String> {
        Ok(self
            .network_generation
            .networks
            .routers
            .router(router)
            .await?
            .map_or_else(String::new, |router| router.public_ip))
    }

    async fn network(&self, router: RouterId) -> Result<NetworkId> {
        Ok(self
            .network_generation
            .networks
            .routers
            .router(router)
            .await?
            .expect("the generated router exists")
            .network)
    }

    async fn network_sender(&self, public_ip: &str) -> Result<String> {
        let Some(router) = self
            .network_generation
            .networks
            .routers
            .router_at(public_ip)
            .await?
        else {
            return Ok(String::new());
        };
        let fallback = router.contact.clone();
        if let Some(person) = self.npcs.network_admin_person(router.network).await?
            && let Some(address) = self.mail.mail_address(person).await?
        {
            return Ok(address);
        }

        Ok(fallback.map_or_else(String::new, |contact| contact.mail))
    }

    /// Creates mission chains without a browser, using the same mission generator as ordinary generated networks.
    pub async fn create_zero_day_chain(&self) -> Result<Option<String>> {
        let Some(group) =
            random_group_for_mission(&self.world.world_seed(), MissionKind::ZeroDay, 0, 0)
        else {
            return Ok(None);
        };
        let public_ip = group[0].clone();
        let Some(router) = self
            .network_generation
            .router_or_generate(&public_ip)
            .await?
        else {
            return Ok(None);
        };
        let network = self
            .network_generation
            .networks
            .routers
            .router(router)
            .await?
            .map(|router| router.network);

        if let Some(network) = network {
            self.install_weak_aptclient(network).await?;
        }

        Ok(Some(public_ip))
    }

    /// Creates a procedural mission at a deterministic seed-selected group, without requiring a mission-shop browser.
    pub async fn create_procedural_mission(&self, seed: u64) -> Result<Option<String>> {
        let kind = MissionKind::Procedural;
        let count = u64::from(kind.count());
        let index = u32::try_from(seed % count).unwrap_or(0);
        let start = u32::try_from((seed / count) % u64::from(GROUP_COUNT)).unwrap_or(0);
        let Some(group) = random_group_for_mission(&self.world.world_seed(), kind, index, start)
        else {
            return Ok(None);
        };
        let public_ip = group[0].clone();

        self.network_generation
            .router_or_generate(&public_ip)
            .await?;

        Ok(Some(public_ip))
    }

    async fn install_weak_aptclient(&self, network: NetworkId) -> Result<bool> {
        let computers: Vec<ComputerId> = self
            .computers
            .computers_in(network)
            .await?
            .into_iter()
            .map(|computer| computer.id)
            .collect();
        let Some(computer) = self.first_service_computer(&computers).await? else {
            return Ok(false);
        };
        let library = weak_aptclient();

        if let Some(node) = self
            .files
            .lookup
            .file(computer, "/lib/aptclient.so", false)
            .await?
        {
            self.generator
                .set_full_library_of(&node, Some(&library))
                .await?;
        } else if let Some(folder) = self.files.lookup.folder(computer, "/lib", false).await? {
            let node = NodeDraft {
                library: Some(Box::new(library)),
                ..NodeDraft::file(
                    "aptclient.so",
                    ROOT_USER,
                    PermissionSet::default_for(false),
                    FileData::library(FileData::binary_size("aptclient.so", true)),
                )
            };
            self.generator.add_full_node(&folder, &node).await?;
        } else {
            return Ok(false);
        }

        Ok(true)
    }

    async fn first_service_computer(&self, computers: &[ComputerId]) -> Result<Option<ComputerId>> {
        for &computer in computers {
            if !self.computers.ports(computer).await?.is_empty() {
                return Ok(Some(computer));
            }
        }

        Ok(computers.first().copied())
    }
}

fn game_text(key: &str) -> String {
    GameTextTable::english().get(key).unwrap_or(key).to_owned()
}

fn conversation_tags(conversation: &str, tag: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Line {
        text: String,
    }

    let mut keys = Vec::new();
    let Some(lines) = GameTextTable::english()
        .get(conversation)
        .and_then(|text| serde_json::from_str::<Vec<Line>>(text).ok())
    else {
        return keys;
    };

    for line in lines {
        let mut rest = line.text.as_str();
        while let Some(index) = rest.find(tag) {
            rest = &rest[index..];
            let end = tag.len() + 1;
            if rest.len() >= end {
                let key = &rest[..end];
                if !keys.iter().any(|existing| existing == key) {
                    keys.push(key.to_owned());
                }
            }
            rest = &rest[end.min(rest.len())..];
        }
    }

    keys
}

fn hide_ip(address: &str, random: &mut Rng) -> String {
    let first = random.range(0, 3);
    let mut second = random.range(0, 2);
    if second >= first {
        second += 1;
    }

    address
        .split('.')
        .enumerate()
        .map(|(index, octet)| {
            if u32::try_from(index).ok() == Some(first) || u32::try_from(index).ok() == Some(second)
            {
                "XXX"
            } else {
                octet
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn weak_aptclient() -> Library {
    let weakness = Weakness {
        remote: true,
        unsec_value: "rvalues".to_owned(),
        details: "zero-day test weakness".to_owned(),
        requirements: Vec::new(),
        reward: Reward::Shell,
        user: GUEST_USER.to_owned(),
        required_lib: None,
        req_lib_version: "1.0.0".to_owned(),
        metaxploit_version: "0.0.9".to_owned(),
        num_register_users: 1,
        num_port_forward: 1,
        num_conn_gateway: 1,
        path_exist: "/usr".to_owned(),
        random_path: "/usr".to_owned(),
    };

    Library {
        kind: LibraryKind::AptClient,
        version: "1.0.0".to_owned(),
        zones: vec![MemoryZone {
            address: "0xA11CE".to_owned(),
            hide: false,
            weaknesses: vec![weakness],
        }],
    }
}

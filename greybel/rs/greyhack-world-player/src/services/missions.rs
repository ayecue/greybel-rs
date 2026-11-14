//! Accepting a procedural mission (`MissionsHelperServer.AcceptMissionServerRpc`).

use greyhack_behaviour::{GROUP_COUNT, MissionKind, kind_for_ip, random_group_for_mission};
use greyhack_world_assets::GameTextTable;
use greyhack_world_core::Result;
use greyhack_world_missions::group_mission;

/// Why a mission was not accepted (`MissionsHelperServer.AcceptMissionServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionFailure {
    /// The address is not a hack shop or police network (`MISSION_ERROR`).
    NotAMissionShop,
    /// The player has no mail account.
    NoMailAccount,
    /// No group of addresses carries a mission for the offer.
    NoMission,
}

/// An accepted procedural mission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedMission {
    /// Public addresses of the networks of the mission, the first the one the contract names.
    pub group: Vec<String>,
}

/// Methods of [`World`] for player missions.
impl MissionOffers {
    pub async fn accept_procedural_mission(
        &self,
        shop_ip: &str,
    ) -> Result<Option<std::result::Result<AcceptedMission, MissionFailure>>> {
        let Some(shop) = self.networks.routers.router_at(shop_ip).await? else {
            return Ok(None);
        };
        let Some(network) = self.networks.settings.network(shop.network).await? else {
            return Ok(None);
        };
        let kind = match network.kind {
            greyhack_behaviour::NetworkKind::HackShop => MissionKind::Procedural,
            greyhack_behaviour::NetworkKind::Police => MissionKind::PoliceProcedural,
            _ => return Ok(Some(Err(MissionFailure::NotAMissionShop))),
        };
        let Some(address) = self.mail.mail_address(self.world.player_person()).await? else {
            return Ok(Some(Err(MissionFailure::NoMailAccount)));
        };
        let mut rng = self.world.world_mut().rng.fork("mission offer");
        let index = rng.range(0, kind.count() - 1);
        let start = rng.range(0, GROUP_COUNT - 1);
        let Some(group) = random_group_for_mission(&self.world.world_seed(), kind, index, start)
        else {
            return Ok(Some(Err(MissionFailure::NoMission)));
        };
        let Some(part) = group_mission(&self.world.world_seed(), &group, kind, index, false)
            .and_then(|mission| mission.parts.into_iter().next())
        else {
            return Ok(Some(Err(MissionFailure::NoMission)));
        };

        self.network_generation
            .router_or_generate(&group[0])
            .await?;

        let placement = self
            .network_generation
            .lan_plan(
                &group[0],
                kind_for_ip(&self.world.world_seed(), &group[0]),
                true,
                false,
            )
            .1
            .and_then(|plan| plan.mission);
        let local_ip = placement
            .as_ref()
            .and_then(|placement| placement.local_ips.first().cloned())
            .unwrap_or_default();
        let npcs = &part.pre_gen_npcs;
        let npc_name = usize::try_from(part.procedural_global_npc_index)
            .ok()
            .and_then(|index| npcs.get(index))
            .or_else(|| npcs.first())
            .map_or_else(|| "unknown".to_owned(), |npc| npc.complete_name());
        let text = GameTextTable::english()
            .get(&part.starter_text_id)
            .unwrap_or(&part.starter_text_id)
            .replace("$PUBLIC_IP", &format!("<b>{}</b>", group[0]))
            .replace("$LOCAL_IP", &format!("<b>{local_ip}</b>"))
            .replace("$IP_LOCAL", &local_ip)
            .replace("$IP_REMOTE", &group[0])
            .replace("$IP_REMLOCAL", &local_ip)
            .replace("$NPC_NAME_GLOBAL", &npc_name)
            .replace("$NPC_NAME_LOCAL", &npc_name)
            .replace("$NPC_NAME_REMOTE", &npc_name);
        let sender = shop
            .contact
            .as_ref()
            .map_or_else(String::new, |contact| contact.mail.clone());

        self.mail
            .deliver_mail(&address, &sender, "Mission Contract", &text)
            .await?;

        Ok(Some(Ok(AcceptedMission { group })))
    }
}

/// A world service with only its declared dependencies.
pub struct MissionOffers {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) network_generation:
        greyhack_world_core::Dep<greyhack_world_creation::NetworkGeneration>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
}

impl MissionOffers {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
        network_generation: greyhack_world_core::Dep<greyhack_world_creation::NetworkGeneration>,
        mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    ) -> Self {
        Self {
            world: world.clone(),
            networks,
            network_generation,
            mail,
        }
    }
}

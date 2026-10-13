//! Accepting a procedural mission (`MissionsHelperServer.AcceptMissionServerRpc`).

use greyhack_behaviour::{GROUP_COUNT, MissionKind, kind_for_ip, random_group_for_mission};
use greyhack_world_assets::GameTextTable;

use crate::generation::missions::{Source, mission_of_group};
use crate::{Result, World};

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

impl World {
    /// Accepts a new procedural mission offered by the hack shop or police network at `shop_ip`, as the shop's
    /// browser does for a procedural offer (`MissionsHelperServer.AddProceduralOffers`, `AcceptMissionServerRpc`):
    /// a group of addresses is drawn for it (`IPGroupManager.GetRandomGroupForMission`), its first network is generated
    /// (`ServerMap.GetRouter`), and the contract (`GetProceduralStarterMessage`) is mailed to the player from the
    /// shop's administrator. Returns `None` when `shop_ip` has no network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn accept_procedural_mission(
        &mut self,
        shop_ip: &str,
    ) -> Result<Option<std::result::Result<AcceptedMission, MissionFailure>>> {
        let Some(shop) = self.router_at(shop_ip).await? else {
            return Ok(None);
        };
        let Some(network) = self.network(shop.network).await? else {
            return Ok(None);
        };
        let (kind, source) = match network.kind {
            greyhack_behaviour::NetworkKind::HackShop => {
                (MissionKind::Procedural, Source::Hackshop)
            }
            greyhack_behaviour::NetworkKind::Police => {
                (MissionKind::PoliceProcedural, Source::Police)
            }
            _ => return Ok(Some(Err(MissionFailure::NotAMissionShop))),
        };
        let Some(address) = self
            .db
            .person(self.player_person)
            .await?
            .and_then(|person| person.mail_address)
        else {
            return Ok(Some(Err(MissionFailure::NoMailAccount)));
        };
        let mut rng = self.rng.fork("mission offer");
        let index = rng.range(0, kind.count() - 1);
        let start = rng.range(0, GROUP_COUNT - 1);
        let Some(group) = random_group_for_mission(&self.world_seed, kind, index, start) else {
            return Ok(Some(Err(MissionFailure::NoMission)));
        };
        let Some((_, mission)) = mission_of_group(&self.world_seed, &group, source) else {
            return Ok(Some(Err(MissionFailure::NoMission)));
        };
        let Some(part) = mission.split().into_iter().next() else {
            return Ok(Some(Err(MissionFailure::NoMission)));
        };

        self.router_or_generate(&group[0]).await?;

        let placement = self
            .lan_plan(&group[0], kind_for_ip(&self.world_seed, &group[0]), true)
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

        self.deliver_mail(&address, &sender, "Mission Contract", &text)
            .await?;

        Ok(Some(Ok(AcceptedMission { group })))
    }
}

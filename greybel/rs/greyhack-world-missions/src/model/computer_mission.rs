use greyhack_behaviour::NetworkKind;

use super::MissionItem;
use crate::config::Archetype;

/// `MissionConfig.ComputerMission`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerMission {
    /// The kind the network takes when the mission sets one (`ComputerMission.tipoRed`, with
    /// `isRandomTipoRed` unset).
    pub network_kind: Option<NetworkKind>,
    pub computer_index_net: i32,
    pub clone_npc_pc_idx: i32,
    pub procedural_archetype: Option<Archetype>,
    pub items: Vec<MissionItem>,
}

impl ComputerMission {
    pub fn new(computer_index_net: i32) -> Self {
        Self {
            network_kind: None,
            computer_index_net,
            clone_npc_pc_idx: -1,
            procedural_archetype: None,
            items: Vec::new(),
        }
    }
}

use greyhack_behaviour::MissionKind;

use super::{ComputerMission, PreGenNpc};

/// `MissionConfig.Mission`: what a group of networks requests of the world, whatever its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    pub kind: MissionKind,
    pub computers: Vec<ComputerMission>,
    pub pre_gen_npcs: Vec<PreGenNpc>,
    pub closure_network_index: i32,
    pub closure_local_index: i32,
    pub starter_text_id: String,
    pub procedural_global_npc_index: i32,
    pub procedural_level: i32,
    pub reward_tier: i32,
}

impl Default for Mission {
    fn default() -> Self {
        Self {
            kind: MissionKind::NoMission,
            computers: Vec::new(),
            pre_gen_npcs: Vec::new(),
            closure_network_index: -1,
            closure_local_index: -1,
            starter_text_id: String::new(),
            procedural_global_npc_index: -1,
            procedural_level: -1,
            reward_tier: -1,
        }
    }
}

impl Mission {
    /// `MissionGen.SplitMission`: the part of each network of the chain, whose computers are those with its network
    /// index.
    pub fn split(&self) -> Vec<Self> {
        let mut parts = Vec::new();
        let mut tail = self.computers.clone();
        let mut network = 0;

        while !tail.is_empty() {
            let mut part = Self {
                computers: Vec::new(),
                // The game's parts do not copy the reward tier.
                reward_tier: -1,
                ..self.clone()
            };
            let mut index = tail.len();

            // The game walks the computers backwards and reverses what it took.
            while index > 0 {
                index -= 1;

                if tail[index].computer_index_net == network {
                    part.computers.push(tail.remove(index));
                }
            }

            part.computers.reverse();

            if !part.computers.is_empty() {
                parts.push(part);
            }

            network += 1;
        }

        parts
    }
}

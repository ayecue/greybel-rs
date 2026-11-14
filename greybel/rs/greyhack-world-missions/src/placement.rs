//! Which part of a mission a public address is and where the mission puts its computers (`NetworkLan.ConfigureMissions`,
//! `NetworkLan.CreateMission`, `NetworkLan.ConfigureChainMission`).

use greyhack_behaviour::{
    FirewallRule, MissionKind, WorldSeed, ip_group, is_valid_ip, mission_for_group,
};

use crate::group::group_mission;
use crate::model::{FirewallAction, FirewallItem, FirewallMatch, ItemKind, Mission, PreGenNpc};

/// The part of a mission a network has (`NetworkLan.missionParts[indexIp]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionRequest {
    /// Addresses of the networks of the mission (`NetworkLan.groupIPs`).
    pub group: Vec<String>,
    /// Position of the network in the group.
    pub index: usize,
    /// Seed of the mission (`MissionGen.GetMissionSeed`).
    pub seed: i32,
    pub part: Mission,
}

impl MissionRequest {
    /// The kind of mission of the group.
    pub fn kind(&self) -> MissionKind {
        self.part.kind
    }

    /// Whether the network is built by `NetworkLan.ConfigureChainMission`: a later network of the chain, or any
    /// network of a procedural or hidden mission (`NetworkLan.CreateMission`). The others only name their NPCs and
    /// put the mission's computers in place.
    pub fn is_chain(&self) -> bool {
        self.index > 0 || self.kind().is_procedural() || self.kind() == MissionKind::Secret
    }

    /// Whether the computers of the mission must be reachable from the main router (`ConfigureChainMission` of a
    /// zero-day mission).
    pub fn needs_reachable_computers(&self) -> bool {
        self.is_chain() && self.kind() == MissionKind::ZeroDay
    }

    /// Whether the network's own generator picks the computers (the first network of the group), rather than the
    /// mission's seed (`NetworkLan.CreateMission`'s selection random).
    pub fn picks_with_network_generator(&self) -> bool {
        self.index == 0
    }

    /// The kind the network takes, where the mission sets one (`NetworkLan.NetworkLan`: `SetTipoRed` to the `tipoRed`
    /// of the first computer, for the later networks of the chain).
    pub fn network_kind(&self) -> Option<greyhack_behaviour::NetworkKind> {
        if self.index == 0 {
            return None;
        }

        self.part
            .computers
            .first()
            .and_then(|computer| computer.network_kind)
    }
}

/// Where a computer of the mission went.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub local_ip: String,
    /// Index into the computers of the part.
    pub computer: usize,
    /// The draw that picks which account of the computer the mission NPC replaces
    /// (`NetworkLan.GetMissionNpcReplacementIndex`), when the mission puts an NPC on it.
    pub replacement: Option<f64>,
    /// The same mission NPC is already on another computer of the network (`NetworkLan.IsMissionNpcAlreadyInLan`).
    pub npc_in_lan: bool,
}

/// What a network was given by its mission.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionPlacement {
    pub request: MissionRequest,
    pub assignments: Vec<Assignment>,
    /// Local addresses of the mission computers in the order of the mission (`NetworkLan.GetLocalIPsIdx`).
    pub local_ips: Vec<String>,
    /// Computers that got a website from the mission, with its text (`LanPcInfo.website`): the computers the mission
    /// gave the http service.
    pub websites: Vec<(String, String)>,
}

impl MissionPlacement {
    pub fn npcs(&self) -> &[PreGenNpc] {
        &self.request.part.pre_gen_npcs
    }

    pub fn assignment_at(&self, local_ip: &str) -> Option<&Assignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.local_ip == local_ip)
    }

    /// The rules the mission adds to the firewall of the main router (`NetworkLan.ConfigurePendingFwItems`): the game
    /// adds every firewall item collected so far after each computer of the part, so the rules of earlier computers
    /// repeat.
    pub fn firewall_rules(&self) -> Vec<FirewallRule> {
        if !self.request.is_chain() {
            return Vec::new();
        }

        let mut queue: Vec<&FirewallItem> = Vec::new();
        let mut rules = Vec::new();

        for computer in &self.request.part.computers {
            for item in &computer.items {
                if let ItemKind::Firewall(firewall) = &item.kind {
                    queue.push(firewall);
                }
            }

            rules.extend(
                queue
                    .iter()
                    .map(|firewall| firewall_rule(firewall, &self.request.group)),
            );
        }

        rules
    }
}

/// `HwFirewallRule.HwFirewallRule` of the values `ConfigurePendingFwItems` gives it.
fn firewall_rule(item: &FirewallItem, group: &[String]) -> FirewallRule {
    let address = |status: FirewallMatch, index: i32| {
        if status == FirewallMatch::Any {
            return None;
        }

        usize::try_from(index)
            .ok()
            .and_then(|index| group.get(index))
            .filter(|address| is_valid_ip(address))
            .cloned()
    };

    FirewallRule {
        allow: item.action == FirewallAction::Allow,
        port: (item.status_port == FirewallMatch::Single)
            .then(|| u16::try_from(item.port).ok().filter(|port| *port > 0))
            .flatten(),
        source: address(item.status_source_ip, item.pc_index_source_ip),
        destination: address(item.status_dest_ip, item.pc_index_dest_ip),
    }
}

/// The mission of the network at `public_ip` (`NetworkLan.ConfigureMissions`): `None` where the address has no
/// mission, or none for its position in the group. `zero_day_open` is whether a zero-day mission is generated now
/// (`ZeroDaySystem.IsRumorTime`).
pub fn request_for(
    seed: &WorldSeed,
    public_ip: &str,
    zero_day_open: bool,
) -> Option<MissionRequest> {
    let group = ip_group(seed, public_ip)?;
    let (kind, index) = mission_for_group(seed, &group)?;
    let mission = group_mission(seed, &group, kind, index, zero_day_open)?;
    let at = group.iter().position(|address| address == public_ip)?;
    let part = mission.parts.into_iter().nth(at)?;

    Some(MissionRequest {
        group,
        index: at,
        seed: mission.seed,
        part,
    })
}

/// `NetworkLan.CreateMission`'s choice of computers: each computer of the part takes one of `available` (the standard
/// computers of the network, in the order of `NetworkLan.GetAllComputers`). `network_pick` is the network generator's
/// draw below a count, used by the first network of the group; the others draw from the mission's seed.
pub fn assign_computers(
    request: &MissionRequest,
    mut available: Vec<String>,
    mut network_pick: impl FnMut(usize) -> usize,
) -> Vec<Assignment> {
    let mut random = mono_random::Random::new(request.seed);
    let mut assignments: Vec<Assignment> = Vec::new();
    let mut placed_npcs: Vec<i32> = Vec::new();

    for (index, computer) in request.part.computers.iter().enumerate() {
        if available.is_empty() {
            break;
        }

        let at = if request.picks_with_network_generator() {
            network_pick(available.len())
        } else {
            random.next_below(available.len())
        };
        let local_ip = available.remove(at);
        let npc = computer.clone_npc_pc_idx;
        let replacement = (npc >= 0).then(|| random.next_double());

        assignments.push(Assignment {
            local_ip,
            computer: index,
            replacement,
            npc_in_lan: npc >= 0 && placed_npcs.contains(&npc),
        });

        if npc >= 0 {
            placed_npcs.push(npc);
        }
    }

    assignments
}

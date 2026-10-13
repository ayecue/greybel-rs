//! Rules of generating a network's devices and people (`NetworkLan`).

use std::ops::RangeInclusive;

use crate::{DeviceKind, NetworkKind, Rng};

/// NPCs a generated network gets (`NetworkLan.AddNpcUser`).
pub const NPCS_PER_NETWORK: RangeInclusive<u32> = 1..=3;

/// Weight of a standard computer when the next device is drawn.
const COMPUTER_WEIGHT: u32 = 70;
/// Weight of each camera or appliance when the next device is drawn.
const GADGET_WEIGHT: u32 = 35;
/// Cameras a network has at most (`NetworkLan.GetRandomTypeDevice`).
const MAX_CAMERAS: u32 = 6;

/// Draws the type of the next device of a fixed-size network: a standard computer at weight 70 and each
/// available camera or appliance at weight 35; residential networks only have computers
/// (`NetworkLan.GetRandomTypeDevice`).
///
/// This follows the game's weights but not its bookkeeping: the game considers the fridge and the
/// microwave taken as soon as they are candidates, this marks them taken only when they are drawn
/// (`fridge` and `microwave` say whether one is taken), and it draws one number in `1..=total` where
/// the game draws from a loot table. Planned networks draw their devices like the game does.
pub fn random_device_kind(
    rng: &mut Rng,
    network_kind: NetworkKind,
    cameras: u32,
    fridge: bool,
    microwave: bool,
) -> DeviceKind {
    if network_kind == NetworkKind::Residential {
        return DeviceKind::Computer;
    }

    let camera = cameras < MAX_CAMERAS;
    let mut total = COMPUTER_WEIGHT;

    total += if camera { GADGET_WEIGHT } else { 0 };
    total += if fridge { 0 } else { GADGET_WEIGHT };
    total += if microwave { 0 } else { GADGET_WEIGHT };

    let mut roll = rng.range(1, total);

    if roll <= COMPUTER_WEIGHT {
        return DeviceKind::Computer;
    }

    roll -= COMPUTER_WEIGHT;

    if camera {
        if roll <= GADGET_WEIGHT {
            return DeviceKind::Camera;
        }

        roll -= GADGET_WEIGHT;
    }

    if !fridge && roll <= GADGET_WEIGHT {
        return DeviceKind::Fridge;
    }

    DeviceKind::Microwave
}

/// Returns the indices of the NPCs that can administer the network, given the computer of each NPC
/// (`NetworkLan.SelectAdmin`): those who share their computer with another NPC, or every NPC when none does.
#[must_use]
pub fn admin_candidates<K: PartialEq>(computers: &[K]) -> Vec<usize> {
    let together: Vec<usize> = (0..computers.len())
        .filter(|&index| {
            computers
                .iter()
                .filter(|other| **other == computers[index])
                .count()
                > 1
        })
        .collect();

    if together.is_empty() {
        (0..computers.len()).collect()
    } else {
        together
    }
}

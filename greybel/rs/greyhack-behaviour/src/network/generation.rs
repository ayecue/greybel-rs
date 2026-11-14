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

/// What the draw of the next device's type depends on (`GlobalLan.camIndex`, `existFridge`,
/// `existMicrowave`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DeviceDraw {
    cameras: u32,
    fridge: bool,
    microwave: bool,
}

impl DeviceDraw {
    /// Returns the cameras created so far (`GlobalLan.camIndex`).
    #[must_use]
    pub fn cameras(&self) -> u32 {
        self.cameras
    }

    /// Counts a created camera (`NetworkLan.PreCrearServidor` increments `GlobalLan.camIndex`).
    pub fn add_camera(&mut self) {
        self.cameras += 1;
    }

    /// Draws the type of the next device (`NetworkLan.GetRandomTypeDevice`): a loot table that gets
    /// a standard computer at weight 70 and, while fewer than six cameras exist and the network is
    /// not residential, a camera, the fridge unless one was offered before and the microwave unless
    /// one was offered before, each at weight 35, in this order. The fridge and the microwave
    /// count as taken as soon as they are candidates, drawn or not. The table is drawn once, also
    /// when the computer is its only entry.
    pub fn draw(&mut self, rng: &mut Rng, network_kind: NetworkKind) -> DeviceKind {
        let mut candidates = vec![(DeviceKind::Computer, COMPUTER_WEIGHT)];

        if self.cameras < MAX_CAMERAS && network_kind != NetworkKind::Residential {
            candidates.push((DeviceKind::Camera, GADGET_WEIGHT));

            if !self.fridge {
                candidates.push((DeviceKind::Fridge, GADGET_WEIGHT));
                self.fridge = true;
            }

            if !self.microwave {
                candidates.push((DeviceKind::Microwave, GADGET_WEIGHT));
                self.microwave = true;
            }
        }

        let total: u32 = candidates.iter().map(|(_, weight)| weight).sum();
        let hit = rng.range(0, total - 1);
        let mut sum = 0;

        for (kind, weight) in candidates {
            sum += weight;

            if hit < sum {
                return kind;
            }
        }

        DeviceKind::Computer
    }
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

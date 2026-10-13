//! Roles and plates drawn for new NPCs (`NpcInfo.NpcInfo`).

use crate::{JobRole, Rng, WorldSeed, license_plate};

/// Selects a role from `roles`, removing project manager after selection
/// (`NetworkLan.GlobalLan.AssignRandomJob`). Falls back to sales when empty.
pub fn assign_role(roles: &mut Vec<JobRole>, rng: &mut Rng) -> JobRole {
    if roles.is_empty() {
        return JobRole::Sales;
    }

    let role = roles[rng.index(roles.len())];

    if role == JobRole::ProjectManager {
        roles.retain(|other| *other != JobRole::ProjectManager);
    }

    role
}

/// Generates a plate from `user` and network `public_ip` for half of NPCs (`NpcInfo.NpcInfo`).
pub fn draw_plate(seed: &WorldSeed, rng: &mut Rng, public_ip: &str, user: &str) -> Option<String> {
    (rng.range(0, 1) == 1)
        .then(|| license_plate(seed, public_ip, user))
        .flatten()
}

//! Seeded library-weakness generation based on `Library` structure. Does not reproduce Grey Hack's
//! exact random stream.

use greyhack_behaviour::{
    FIRST_VERSION, GUEST_USER, LibraryKind, LibraryRole, ROOT_USER, Requirement, Reward,
};

use crate::{Library, MemoryZone, Rng, Weakness, Word};

/// Folders exposed by random-folder weakness rewards.
pub(super) const PATHS: &[&str] = &[
    "/etc", "/lib", "/bin", "/usr", "/usr/bin", "/home", "/root", "/sys", "/boot",
];
/// Paths referenced by `PathExist` requirements.
pub(super) const PATH_EXISTS: &[&str] = &["/Public", "/usr", "/bin", "/lib/aptclient.so"];
pub(super) const DETAILS: &[&str] = &["loop in array", "string copy in"];
/// Libraries referenced by `Library` requirements.
pub(super) const REQUIRED_LIBS: &[LibraryKind] = &[
    LibraryKind::KernelModule,
    LibraryKind::Init,
    LibraryKind::Net,
];

impl Library {
    /// Generates a named, versioned library with weaknesses and hidden memory zones using `rng`.
    #[must_use]
    pub fn generate(rng: &mut Rng, kind: LibraryKind, version: &str) -> Self {
        let mut library = Self {
            kind,
            version: version.to_owned(),
            zones: Vec::new(),
        };
        let router = kind.role() == LibraryRole::Router;
        let addresses = if router {
            rng.range(1, 2)
        } else {
            rng.range(1, 4)
        };

        for index in 0..addresses {
            let zone = memory_zone(&library, rng, kind, false);

            library.zones.push(zone);
            if router && index == addresses - 1 {
                force_router_weakness(&mut library, rng);
            }
        }

        if !router {
            let hidden_count = rng.range(2, 4);

            for _ in 0..hidden_count {
                let zone = memory_zone(&library, rng, kind, true);

                library.zones.push(zone);
            }
        }

        library
    }
}

fn unique_address(library: &Library, rng: &mut Rng) -> String {
    loop {
        let address = format!("0x{:X}", rng.next_u64() as u32);

        if !library.zones.iter().any(|zone| zone.address == address) {
            return address;
        }
    }
}

fn memory_zone(library: &Library, rng: &mut Rng, kind: LibraryKind, hide: bool) -> MemoryZone {
    let address = unique_address(library, rng);
    let count = rng.range(2, 5);
    let mut weaknesses = Vec::new();

    for _ in 0..count {
        let weakness = generate_weakness(rng, kind, hide, false, &weaknesses);

        weaknesses.push(weakness);
    }

    MemoryZone {
        address,
        hide,
        weaknesses,
    }
}

/// Ensures the router has an unhidden guest shell or computer weakness with no requirements; adds
/// one if absent.
fn force_router_weakness(library: &mut Library, rng: &mut Rng) {
    let satisfied = library.zones.iter().any(|zone| {
        !zone.hide
            && zone.weaknesses.iter().any(|weakness| {
                matches!(weakness.reward, Reward::Shell | Reward::Computer)
                    && weakness.user == GUEST_USER
                    && weakness.requirements.is_empty()
            })
    });

    if !satisfied && let Some(zone) = library.zones.last_mut() {
        let weakness = generate_weakness(
            rng,
            LibraryKind::KernelRouter,
            false,
            true,
            &zone.weaknesses,
        );

        zone.weaknesses.push(weakness);
    }
}

/// Configures a weakness for library `kind` (`Vulnerabilidad.Configure`). Remote weaknesses can
/// require network conditions; local weaknesses cannot and never grant guest shells.
fn generate_weakness(
    rng: &mut Rng,
    kind: LibraryKind,
    hide: bool,
    force_router: bool,
    zone: &[Weakness],
) -> Weakness {
    let router = kind.role() == LibraryRole::Router;
    let remote = kind.is_remote();
    let mut rewards = vec![Reward::Computer, Reward::Shell, Reward::RandomFolder];

    rewards.push(if router {
        Reward::FirewallDisable
    } else {
        Reward::ChangePass
    });

    match kind {
        LibraryKind::SmartAppliance => rewards.push(Reward::OverrideSettings),
        LibraryKind::TrafficNet => rewards.push(Reward::TrafficLightControl),
        _ => {}
    }

    let reward = if router && force_router {
        *rng.pick(&[Reward::Shell, Reward::Computer])
    } else {
        pick_reward(rng, &rewards)
    };

    let force_guest = force_router
        || (router
            && matches!(
                reward,
                Reward::Shell | Reward::RandomFolder | Reward::FirewallDisable
            ));

    let mut allowed_requirements = if router {
        vec![Requirement::LocalNetwork, Requirement::ConnGateway]
    } else {
        vec![
            Requirement::Library,
            Requirement::NumberUsersRegister,
            Requirement::AnyActiveUser,
            Requirement::RootActiveUser,
            Requirement::PortForward,
            Requirement::ConnGateway,
            Requirement::GuestActiveUser,
            Requirement::PathExist,
        ]
    };

    if force_guest {
        allowed_requirements.clear();
    } else if !remote {
        allowed_requirements.retain(|requirement| {
            !matches!(
                requirement,
                Requirement::ConnGateway | Requirement::PortForward | Requirement::LocalNetwork
            )
        });
    } else if !router {
        allowed_requirements.retain(|requirement| {
            !matches!(
                requirement,
                Requirement::ConnGateway | Requirement::LocalNetwork
            )
        });
    }

    let count = if force_guest {
        0
    } else if router {
        *rng.pick(&[0u32, 1])
    } else {
        required_count(rng, allowed_requirements.len())
    };

    let mut requirements: Vec<Requirement> = Vec::new();
    let mut required_lib = None;
    let mut req_lib_version = String::new();

    for _ in 0..count {
        if allowed_requirements.is_empty() {
            break;
        }

        let action = rng.take(&mut allowed_requirements);

        if action == Requirement::Library {
            let libs: Vec<LibraryKind> = REQUIRED_LIBS
                .iter()
                .filter(|candidate| **candidate != kind)
                .copied()
                .collect();

            required_lib = Some(*rng.pick(&libs));
            req_lib_version = FIRST_VERSION.to_owned();
        }

        if should_add(&requirements, action) {
            requirements.push(action);
        }
    }

    let mut user = pick_user(rng, hide);

    if force_guest {
        GUEST_USER.clone_into(&mut user);
    }

    if user == GUEST_USER && (reward == Reward::ChangePass || !remote) {
        user = "normal_user".to_owned();
    }

    // A zone's weaknesses have distinct values; a repeated one is drawn again.
    let unsec_value = loop {
        let value = Word::LibraryVariable.generate(rng).to_lowercase();

        if !zone.iter().any(|weakness| weakness.unsec_value == value) {
            break value;
        }
    };

    Weakness {
        remote,
        unsec_value,
        details: (*rng.pick(DETAILS)).to_owned(),
        requirements,
        reward,
        user,
        required_lib,
        req_lib_version,
        metaxploit_version: "0.0.9".to_owned(),
        num_register_users: rng.range(1, 2),
        num_port_forward: rng.range(1, 3),
        num_conn_gateway: rng.range(1, 3),
        path_exist: (*rng.pick(PATH_EXISTS)).to_owned(),
        random_path: (*rng.pick(PATHS)).to_owned(),
    }
}

/// Users granted by weaknesses (`GetUserSelected`).
pub(super) const USERS: [&str; 3] = [GUEST_USER, "normal_user", ROOT_USER];
/// [`USERS`] weights for weaknesses in non-hidden memory zones.
pub(super) const USER_WEIGHTS: [u32; 3] = [70, 22, 8];

/// Returns `GetReward` weight for `reward`; shell has the highest weight.
pub(super) const fn reward_weight(reward: Reward) -> u32 {
    match reward {
        Reward::Shell => 55,
        Reward::OverrideSettings => 75,
        Reward::TrafficLightControl => 50,
        _ => 45,
    }
}

/// Selects a reward using `GetReward` weights.
fn pick_reward(rng: &mut Rng, results: &[Reward]) -> Reward {
    let weights: Vec<u32> = results.iter().copied().map(reward_weight).collect();

    results[pick_weighted_index(rng, &weights)]
}

/// Selects a weakness user using `GetUserSelected` weights. Guest is most likely; `hide` changes
/// the weights.
fn pick_user(rng: &mut Rng, hide: bool) -> String {
    let weights = if hide { [60, 25, 15] } else { USER_WEIGHTS };

    USERS[pick_weighted_index(rng, &weights)].to_owned()
}

/// Selects 0 through `min(max, 4)` requirements using `GetRequiredActions` weights.
fn required_count(rng: &mut Rng, max: usize) -> u32 {
    let max = max.min(4);

    if max == 0 {
        return 0;
    }

    let weights: Vec<u32> = (0..max)
        .map(|i| if i == 0 { 30 } else { (100 - max * 8) as u32 })
        .collect();

    pick_weighted_index(rng, &weights) as u32
}

/// Prevents `RootActiveUser` and `AnyActiveUser` from appearing in the same requirement set.
pub(super) fn should_add(requirements: &[Requirement], action: Requirement) -> bool {
    if action != Requirement::RootActiveUser && action != Requirement::AnyActiveUser {
        return true;
    }

    !requirements.contains(&Requirement::RootActiveUser)
        && !requirements.contains(&Requirement::AnyActiveUser)
}

/// Selects an index proportional to `weights`, matching one `RDSTable` pick. Returns 0 if all
/// weights are zero.
fn pick_weighted_index(rng: &mut Rng, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();

    if total == 0 {
        return 0;
    }

    let mut roll = rng.range(0, total - 1);

    for (index, weight) in weights.iter().enumerate() {
        if roll < *weight {
            return index;
        }

        roll -= *weight;
    }

    weights.len() - 1
}

#[cfg(test)]
#[path = "tests/library.rs"]
mod tests;

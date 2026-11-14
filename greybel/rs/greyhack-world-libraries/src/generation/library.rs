//! Seeded library-weakness generation based on `Library` structure. Does not reproduce Grey Hack's
//! exact random stream.

use std::collections::HashMap;

use greyhack_behaviour::{
    FIRST_VERSION, GUEST_USER, LibraryKind, LibraryRole, ROOT_USER, Requirement, Reward,
};

use crate::{Library, MemoryZone, Rng, Weakness, Word};

/// Metaxploit version a weakness needs to be seen when the library is made with none
/// (`Library.Vulnerabilidad.metaxploitVersion`).
const DEFAULT_METAXPLOIT_VERSION: &str = "0.0.9";

/// The versions the world stores of each library kind (`VersionsControl.allLibs`), from which a weakness that requires
/// a library draws the version it requires (`Library.Vulnerabilidad.Configure`).
pub type VersionTable = HashMap<LibraryKind, Vec<String>>;

/// What the game knows while it makes weaknesses, beyond the generator.
#[derive(Clone, Copy, Default)]
pub struct WeaknessContext<'a> {
    /// Metaxploit version the weaknesses need to be seen, or none for the default
    /// (`Library.Vulnerabilidad.Configure`).
    pub(crate) metaxploit: Option<&'a str>,
    /// The stored versions of every library kind, once the world has configured them
    /// (`VersionsControl.LibsConfigured`).
    pub(crate) table: Option<&'a VersionTable>,
}

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

/// Methods of [`Library`] for generation library.
pub trait LibraryGeneration {
    /// Generates a named, versioned library with weaknesses and hidden memory zones using `rng`.
    #[must_use]
    fn generate(rng: &mut Rng, kind: LibraryKind, version: &str) -> Self;

    /// Adds `count` memory zones, hidden or not, each with weaknesses; the last zone of a router library gets the
    /// weakness every router needs when none has it (`Library.AddMemZone`).
    fn add_zones(&mut self, rng: &mut Rng, count: u32, hide: bool, context: WeaknessContext<'_>);

    /// Adds the hidden memory zones of a library that is not a router's (`Library.ConfigHideMemZones`).
    fn add_hidden_zones(&mut self, rng: &mut Rng, context: WeaknessContext<'_>);

    /// Removes the hidden memory zones of a library that is not a router's (`Library.DeleteHideMemZones`).
    fn remove_hidden_zones(&mut self);

    /// Returns the index of a random zone, hidden or not (`Library.GetRandomMem`).
    fn random_zone(&self, rng: &mut Rng, hide: bool) -> Option<usize>;

    /// Gives the weakness at `index` of `zone` new contents, as the weaknesses of a patched version have
    /// (`Library.Vulnerabilidad.Configure`).
    fn renew_weakness(
        &mut self,
        rng: &mut Rng,
        zone: usize,
        index: usize,
        context: WeaknessContext<'_>,
    );

    /// Ensures the router has an unhidden guest shell or computer weakness with no requirements, adding one to the zone
    /// at `zone` if no zone has it (`Library.ForceRouterVulns`).
    fn force_router_weakness(&mut self, rng: &mut Rng, zone: usize, context: WeaknessContext<'_>);
}

impl LibraryGeneration for Library {
    fn generate(rng: &mut Rng, kind: LibraryKind, version: &str) -> Self {
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

        let context = WeaknessContext::default();

        library.add_zones(rng, addresses, false, context);
        library.add_hidden_zones(rng, context);

        library
    }

    fn add_zones(&mut self, rng: &mut Rng, count: u32, hide: bool, context: WeaknessContext<'_>) {
        for index in 0..count {
            let zone = memory_zone(self, rng, hide, context);

            self.zones.push(zone);
            if self.kind.role() == LibraryRole::Router && index == count - 1 {
                self.force_router_weakness(rng, self.zones.len() - 1, context);
            }
        }
    }

    fn add_hidden_zones(&mut self, rng: &mut Rng, context: WeaknessContext<'_>) {
        if self.kind.role() == LibraryRole::Router {
            return;
        }

        let count = rng.range(2, 4);

        self.add_zones(rng, count, true, context);
    }

    fn remove_hidden_zones(&mut self) {
        if self.kind.role() != LibraryRole::Router {
            self.zones.retain(|zone| !zone.hide);
        }
    }

    fn random_zone(&self, rng: &mut Rng, hide: bool) -> Option<usize> {
        let zones: Vec<usize> = (0..self.zones.len())
            .filter(|&index| self.zones[index].hide == hide)
            .collect();

        (!zones.is_empty()).then(|| zones[rng.index(zones.len())])
    }

    fn renew_weakness(
        &mut self,
        rng: &mut Rng,
        zone: usize,
        index: usize,
        context: WeaknessContext<'_>,
    ) {
        let weakness = generate_weakness(
            rng,
            self.kind,
            self.zones[zone].hide,
            false,
            &self.zones[zone].weaknesses,
            context,
        );

        self.zones[zone].weaknesses[index] = weakness;
    }

    fn force_router_weakness(&mut self, rng: &mut Rng, zone: usize, context: WeaknessContext<'_>) {
        let satisfied = self.zones.iter().any(|zone| {
            !zone.hide
                && zone.weaknesses.iter().any(|weakness| {
                    matches!(weakness.reward, Reward::Shell | Reward::Computer)
                        && weakness.user == GUEST_USER
                        && weakness.requirements.is_empty()
                })
        });

        if !satisfied {
            let weakness = generate_weakness(
                rng,
                LibraryKind::KernelRouter,
                false,
                true,
                &self.zones[zone].weaknesses,
                context,
            );

            self.zones[zone].weaknesses.push(weakness);
        }
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

fn memory_zone(
    library: &Library,
    rng: &mut Rng,
    hide: bool,
    context: WeaknessContext<'_>,
) -> MemoryZone {
    let address = unique_address(library, rng);
    let count = rng.range(2, 5);
    let mut weaknesses = Vec::new();

    for _ in 0..count {
        let weakness = generate_weakness(rng, library.kind, hide, false, &weaknesses, context);

        weaknesses.push(weakness);
    }

    MemoryZone {
        address,
        hide,
        weaknesses,
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
    context: WeaknessContext<'_>,
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

            let lib = *rng.pick(&libs);

            required_lib = Some(lib);
            req_lib_version = match context.table.and_then(|table| table.get(&lib)) {
                Some(versions) if !versions.is_empty() => rng.pick(versions).clone(),
                _ => FIRST_VERSION.to_owned(),
            };
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
        metaxploit_version: context
            .metaxploit
            .unwrap_or(DEFAULT_METAXPLOIT_VERSION)
            .to_owned(),
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
pub(crate) mod tests;

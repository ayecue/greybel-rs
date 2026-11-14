//! Derives debug-library zero-days from seeds using Grey Hack's .NET random stream.

use greyhack_behaviour::{
    FIRST_VERSION, GUEST_USER, LibraryKind, ROOT_USER, Requirement, Reward, path,
};
use mono_random::Random;

use super::library::{
    DETAILS, PATH_EXISTS, PATHS, REQUIRED_LIBS, USER_WEIGHTS, USERS, reward_weight, should_add,
};
use crate::{DebugLibraryState, Weakness, Word, ZeroDay, ZeroDayObject, ZeroDayObjectKind};

impl DebugLibraryState {
    /// Seed-derived zero-day: vulnerable address, exploit result objects, and weakness.
    #[must_use]
    pub fn zero_day(&self) -> ZeroDay {
        let mut random = Random::new(self.seed as i32);
        let address = format!("0x{:X}", random.next_int());

        ZeroDay {
            address,
            objects: objects_for(self.seed),
            vulnerability: vulnerability(&self.path, &mut random),
        }
    }
}

const ROOT_METHODS: &[&str] = &[
    "create_folder",
    "create_group",
    "delete_group",
    "create_user",
    "delete_user",
];

fn random_selection(random: &mut Random, original: &[&str]) -> Vec<String> {
    let mut pool = original.to_vec();
    let count = random.next_below(pool.len()) + 1;

    (0..count)
        .map(|_| pool.remove(random.next_below(pool.len())).to_owned())
        .collect()
}

fn objects_for(seed: i64) -> Vec<ZeroDayObject> {
    let mut random = Random::new(seed as i32);

    [
        (
            ZeroDayObjectKind::Computer,
            &[
                "create_folder",
                "touch",
                "create_user",
                "delete_user",
                "create_group",
                "delete_group",
            ][..],
        ),
        (
            ZeroDayObjectKind::File,
            &[
                "move",
                "rename",
                "delete",
                "set_group",
                "set_owner",
                "chmod",
            ][..],
        ),
    ]
    .into_iter()
    .map(|(kind, methods)| {
        let methods = random_selection(&mut random, methods);
        let access = if methods
            .iter()
            .any(|method| ROOT_METHODS.contains(&method.as_str()))
        {
            ROOT_USER
        } else {
            USERS[random.next_below(USERS.len())]
        };

        ZeroDayObject {
            kind,
            methods,
            access: access.to_owned(),
        }
    })
    .collect()
}

fn pick_by_weight(random: &mut Random, weights: &[f32]) -> usize {
    let total: f32 = weights.iter().sum();
    let mut value = random.next_double() as f32 * total;

    for (index, weight) in weights.iter().enumerate() {
        value -= weight;
        if value < 0.0 {
            return index;
        }
    }

    weights.len().saturating_sub(1)
}

fn vulnerability(path: &str, random: &mut Random) -> Weakness {
    let kind = LibraryKind::of_file(path::file_name(path));
    let router = kind == Some(LibraryKind::KernelRouter);
    let remote = kind.is_some_and(LibraryKind::is_remote);
    let mut results = if router {
        vec![
            Reward::Computer,
            Reward::Shell,
            Reward::RandomFolder,
            Reward::FirewallDisable,
        ]
    } else {
        vec![
            Reward::Computer,
            Reward::Shell,
            Reward::RandomFolder,
            Reward::ChangePass,
        ]
    };

    match kind {
        Some(LibraryKind::SmartAppliance) => results.push(Reward::OverrideSettings),
        Some(LibraryKind::TrafficNet) => results.push(Reward::TrafficLightControl),
        _ => {}
    }

    let weights = results
        .iter()
        .map(|reward| reward_weight(*reward) as f32)
        .collect::<Vec<f32>>();
    let reward = results[pick_by_weight(random, &weights)];
    let no_requirements = router
        && matches!(
            reward,
            Reward::Shell | Reward::RandomFolder | Reward::FirewallDisable
        );

    let mut candidates = if router {
        vec![Requirement::LocalNetwork, Requirement::ConnGateway]
    } else {
        vec![
            Requirement::Library,
            Requirement::NumberUsersRegister,
            Requirement::AnyActiveUser,
            Requirement::RootActiveUser,
            Requirement::LocalNetwork,
            Requirement::PortForward,
            Requirement::ConnGateway,
            Requirement::GuestActiveUser,
            Requirement::PathExist,
        ]
    };

    if no_requirements {
        candidates.clear();
    } else {
        candidates.retain(|requirement| {
            !matches!(
                requirement,
                Requirement::RootActiveUser
                    | Requirement::AnyActiveUser
                    | Requirement::GuestActiveUser
                    | Requirement::PortForward
            )
        });

        if !remote {
            candidates.retain(|requirement| {
                !matches!(
                    requirement,
                    Requirement::ConnGateway | Requirement::PortForward | Requirement::LocalNetwork
                )
            });
        } else if !router {
            candidates.retain(|requirement| {
                !matches!(
                    requirement,
                    Requirement::ConnGateway | Requirement::LocalNetwork
                )
            });
        }
    }

    let mut requirements = Vec::new();
    let mut required_lib = None;
    let req_lib_version = FIRST_VERSION.to_owned();

    while !candidates.is_empty() {
        let index = pick_by_weight(random, &vec![50.0; candidates.len()]);
        let requirement = candidates.remove(index);

        if requirement == Requirement::Library {
            required_lib = Some(REQUIRED_LIBS[random.next_below(REQUIRED_LIBS.len())]);
        }

        if should_add(&requirements, requirement) {
            requirements.push(requirement);
        }
    }

    let unsec_value = Word::LibraryVariable
        .generate_with_index(|length| random.next_below(length))
        .to_lowercase();

    // Grey Hack draws a user but assigns the weakness to guest regardless.
    pick_by_weight(random, &USER_WEIGHTS.map(|weight| weight as f32));

    let num_register_users = random.next_below(2) as u32 + 1;
    let num_port_forward = random.next_below(3) as u32 + 1;
    let num_conn_gateway = random.next_below(3) as u32 + 1;
    let path_exist = PATH_EXISTS[random.next_below(PATH_EXISTS.len())].to_owned();
    let random_path = PATHS[random.next_below(PATHS.len())].to_owned();
    let details = DETAILS[random.next_below(DETAILS.len())].to_owned();

    Weakness {
        remote,
        unsec_value,
        details,
        requirements,
        reward,
        user: GUEST_USER.to_owned(),
        required_lib,
        req_lib_version,
        metaxploit_version: "0.0.0".to_owned(),
        num_register_users,
        num_port_forward,
        num_conn_gateway,
        path_exist,
        random_path,
    }
}

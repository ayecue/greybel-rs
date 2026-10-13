//! Generates computers, users, services, library weaknesses, and LAN addresses.

use greyhack_behaviour::{
    COMPUTER_SERVICES, DeviceKind, FIRST_VERSION, FileKind, KERNEL_ROUTER_PATH, LIBRARY_FOLDER,
    LibraryKind, PermissionSet, ROOT_USER, ROUTER_PORT, ServiceId, lan_hosts, library_path,
};

use super::filesystem::{add_libraries, build_filesystem};
use super::preset::system_processes;
use super::{ComputerOptions, library_version};
use crate::model::{STANDARD_DISK_MB, STANDARD_RAM_MB};
use crate::{
    ComputerDraft, Hardware, Library, NetCard, Part, Port, Rng, SmartApplianceSettings, User, Word,
};

/// Computer name shared by routers and switches.
pub(crate) const ROUTER_NAME: &str = "router";

const MAX_SERVICES: u32 = 3;
const MIN_GENERATED_USERS: u32 = 1;
const MAX_GENERATED_USERS: u32 = 3;
const NETWORK_CARD: &str = "Realtek RTL8111";

/// Chooses a LAN address for a new computer behind `router_ip`. Returns `choice` if it is unused
/// and in the router subnet; otherwise draws an unused address from `used_addresses` with `rng`.
/// Returns `None` if no address is available.
pub(crate) fn lan_address(
    rng: &mut Rng,
    router_ip: &str,
    used_addresses: &[String],
    choice: Option<&String>,
) -> Option<String> {
    match choice {
        Some(address)
            if used_addresses.contains(address)
                || *address == router_ip
                || !lan_hosts(router_ip).any(|host| host == *address) =>
        {
            None
        }
        Some(address) => Some(address.clone()),
        None => {
            let unused_addresses = lan_hosts(router_ip)
                .filter(|address| !used_addresses.contains(address))
                .collect::<Vec<_>>();

            (!unused_addresses.is_empty()).then(|| rng.pick(&unused_addresses).clone())
        }
    }
}

/// Builds computer `id` at `lan_ip` from `options` and `rng`, including users, services, ports,
/// files, and libraries. Also returns the router forwards for its service ports.
pub(crate) fn computer(
    rng: &mut Rng,
    lan_ip: &str,
    options: &ComputerOptions,
    device_kind: DeviceKind,
) -> (ComputerDraft, Vec<Port>) {
    let name = options
        .name
        .clone()
        .unwrap_or_else(|| Word::Username.generate(rng));
    let root_password = options
        .root_password
        .clone()
        .unwrap_or_else(|| Word::Password.generate(rng));
    let users = options.users.clone().unwrap_or_else(|| match device_kind {
        DeviceKind::Camera => vec![("cctv".to_owned(), Word::Password.generate(rng))],
        DeviceKind::Microwave | DeviceKind::Fridge => {
            vec![("iot".to_owned(), Word::Password.generate(rng))]
        }
        _ => {
            let count = rng.range(MIN_GENERATED_USERS, MAX_GENERATED_USERS) as usize;
            let mut users: Vec<(String, String)> = Vec::with_capacity(count);

            while users.len() < count {
                let name = Word::Surname.generate(rng);

                if name != ROOT_USER
                    && users
                        .iter()
                        .all(|(used_addresses, _)| *used_addresses != name)
                {
                    users.push((name, Word::Password.generate(rng)));
                }
            }

            users
        }
    });
    let services: Vec<(ServiceId, u16)> = match &options.services {
        Some(names) => names
            .iter()
            .filter_map(|name| {
                ServiceId::from_name(name).map(|service| (service, service.default_port()))
            })
            .collect(),
        None => {
            let device_services: &[ServiceId] = match device_kind {
                DeviceKind::Camera => &[ServiceId::Http, ServiceId::Cam],
                DeviceKind::Microwave | DeviceKind::Fridge => &[ServiceId::SmartAppliance],
                DeviceKind::Smartphone => &[],
                _ => COMPUTER_SERVICES,
            };
            let mut pool = device_services.to_vec();
            let service_list = if matches!(device_kind, DeviceKind::Computer) {
                (0..rng.range(1, MAX_SERVICES))
                    .map(|_| rng.take(&mut pool))
                    .collect()
            } else {
                pool
            };

            service_list
                .into_iter()
                .map(|service| (service, service.default_port()))
                .collect()
        }
    };
    let mut computer = ComputerDraft::new(&name, lan_ip);

    computer.kind = if options.is_switch {
        DeviceKind::Switch
    } else {
        device_kind
    };
    if matches!(device_kind, DeviceKind::Microwave | DeviceKind::Fridge) {
        let prefix = if device_kind == DeviceKind::Fridge {
            "FR"
        } else {
            "MI"
        };
        let model = format!(
            "{}/{prefix}{}",
            Word::Corporation.generate(rng),
            rng.next_u64()
        );
        let required_power = rng.range(600, 1500) as i32;
        let required_temperature = rng.range(20, 60) as i32;

        computer.smart = Some(SmartApplianceSettings::new(
            &model,
            required_power,
            required_temperature,
        ));
    }

    computer.hardware = Hardware::standard(
        STANDARD_RAM_MB,
        STANDARD_DISK_MB,
        &options.mining_hardware,
        vec![NetCard {
            part: Part::new(NETWORK_CARD),
            active: true,
            ..NetCard::new(false, false)
        }],
    );
    computer.users = std::iter::once(User::new(ROOT_USER, &root_password))
        .chain(users.iter().map(|(user, password)| {
            let mut user = User::new(user, password);

            user.deletable = true;

            user
        }))
        .collect();
    computer.processes = system_processes();
    build_filesystem(&mut computer);
    add_libraries(
        &mut computer,
        services.iter().map(|(service, _)| service.library()),
    );
    assign_libraries(&mut computer, rng);

    let mut used_ports = std::collections::BTreeSet::new();
    let ports: Vec<Port> = services
        .into_iter()
        .map(|(service, number)| {
            let mut number = number;

            while !used_ports.insert(number) {
                number = number.wrapping_add(1);
            }

            Port {
                number,
                service,
                version: service_version(&computer, service),
                closed: false,
                visible: true,
            }
        })
        .collect();

    computer.ports = ports.clone();
    if device_kind == DeviceKind::Computer && rng.range(0, 1) == 1 {
        let user = computer
            .users
            .get(1)
            .map_or(ROOT_USER, |user| user.name.as_str())
            .to_owned();

        computer.active_users.push(user);
    }

    (computer, ports)
}

/// Builds a router or switch computer (`Router`, `LanRouter`) with root account `root_password`,
/// router layout, libraries, and `kernel` as `kernel_router.so`. Includes the closed hidden HTTP
/// service at `http_version` used by `Router.RouterConfig`, with its installation files.
#[must_use]
pub fn router_computer(
    rng: &mut Rng,
    kind: DeviceKind,
    local_ip: &str,
    root_password: &str,
    kernel: Library,
    http_version: &str,
) -> ComputerDraft {
    let mut computer = ComputerDraft {
        kind,
        users: vec![User::new(ROOT_USER, root_password)],
        ..ComputerDraft::new(ROUTER_NAME, local_ip)
    };

    build_filesystem(&mut computer);
    add_libraries(&mut computer, [ServiceId::Http.library()]);

    let fs = &mut computer.filesystem;

    if let Some(file) = fs.file(KERNEL_ROUTER_PATH, false) {
        fs.node_mut(file).library = Some(Box::new(kernel));
    }

    if let Some(file) = fs.file(&library_path(ServiceId::Http.library()), false) {
        let node = fs.node_mut(file);

        // Service libraries are executable (`Computer.PostConfigServices`).
        node.permissions = PermissionSet::parse("-rwxr-xr-x").expect("valid permissions");
        node.library = Some(Box::new(Library::generate(
            rng,
            LibraryKind::Http,
            http_version,
        )));
    }

    assign_libraries(&mut computer, rng);
    computer.ports.push(Port {
        number: ROUTER_PORT,
        service: ServiceId::Http,
        version: http_version.to_owned(),
        closed: true,
        visible: false,
    });

    computer
}

/// Generates versions and weaknesses for every `Library` file in `/lib`.
fn assign_libraries(computer: &mut ComputerDraft, rng: &mut Rng) {
    assign_libraries_with(computer, rng, library_version);
}

/// Generates weaknesses for every `Library` file in the player `/lib`, all at the first version.
pub(super) fn assign_player_libraries(computer: &mut ComputerDraft, rng: &mut Rng) {
    assign_libraries_with(computer, rng, |_| FIRST_VERSION.to_owned());
}

/// Generates weaknesses for `/lib` `Library` files that lack them, using the version selected by
/// `version`.
fn assign_libraries_with(
    computer: &mut ComputerDraft,
    rng: &mut Rng,
    version: impl Fn(&mut Rng) -> String,
) {
    let fs = &mut computer.filesystem;
    let Some(lib) = fs.folder(LIBRARY_FOLDER, false) else {
        return;
    };

    for id in fs.files(lib).to_vec() {
        let node = fs.node(id);
        let is_library = node
            .data()
            .is_some_and(|data| data.kind == FileKind::Library)
            && node.library.is_none();
        let Some(kind) = LibraryKind::of_file(&node.name).filter(|_| is_library) else {
            continue;
        };
        let version = version(rng);
        let library = Library::generate(rng, kind, &version);

        fs.node_mut(id).library = Some(Box::new(library));
    }
}

/// Returns the `/lib` version of the library used by `service` on `computer`.
fn service_version(computer: &ComputerDraft, service: ServiceId) -> String {
    computer
        .filesystem
        .library(&library_path(service.library()), false)
        .map_or_else(
            || FIRST_VERSION.to_owned(),
            |library| library.version.clone(),
        )
}

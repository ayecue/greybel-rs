//! A simulated network like the game's: the [`NetworkLayout`] of a [`NetworkSnapshot`] the bridge read.

use greyhack_behaviour::{
    Command, DeviceKind, FIRST_VERSION, FileData, FileKind, FirewallRule, LibraryKind,
    NetworkAdmin, NetworkKind, PermissionSet, Requirement, Reward, SAFE_MODE_TERMINAL,
    SAFE_MODE_TERMINAL_RAM_MB, ServiceId, Side, SmartApplianceSettings, SubscriptionKind,
    WorkPerformance,
};
use greyhack_test_support_client::protocol::test_support::{
    ComputerSnapshot, FileSnapshot, FirewallRuleSnapshot, LanNodeKind, NetworkSnapshot,
    PortSnapshot, RouterSnapshot, TablePortSnapshot,
};
use greyhack_world::World;
use greyhack_world_core::{
    Hardware, MinerHardware, NetCard, Part, Port, Process, SavedNetwork, User,
};
use greyhack_world_creation::{DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout};
use greyhack_world_economy::{
    BankLayout, BankSubscription, CoinLayout, EconomyLayout, OfferLayout, PriceRecord,
    SubWalletLayout, WalletLayout,
};
use greyhack_world_libraries::{
    ComputerDraft, FileSystemDraft, Library, MemoryZone, NodeDraft, RouterDraft, Weakness,
};
use serde::Deserialize;

/// The layout of the network `snapshot` read, node by node in its order.
pub fn layout(snapshot: &NetworkSnapshot) -> NetworkLayout {
    NetworkLayout {
        kind: NetworkKind::of_game_name(&snapshot.kind).unwrap_or(NetworkKind::Unknown),
        public_ip: snapshot.public_ip.clone(),
        // The snapshot has the web address; the domain is what follows its `www.` (`ServerMap.RouterNode.GetDomain`).
        domain: snapshot
            .domain
            .strip_prefix("www.")
            .unwrap_or(&snapshot.domain)
            .to_owned(),
        disabled: snapshot.disabled,
        net_speed: snapshot.net_speed,
        rental_admin: snapshot.rental_admin,
        admin: snapshot.admin.as_ref().map(|admin| NetworkAdmin {
            intelligence: admin.intelligence,
            pc_experience: admin.pc_experience,
            work_performance: WorkPerformance::from_code(admin.work_performance)
                .expect("the game's work performance"),
            dead_or_fired: admin.dead_or_fired,
        }),
        nodes: snapshot
            .nodes
            .iter()
            .map(|node| LanNodeLayout {
                parent: node.parent.map(|parent| parent as usize),
                gateway: node.gateway.clone(),
                device: match &node.node {
                    LanNodeKind::Router(router) => router_layout(&node.local_ip, router),
                    LanNodeKind::Hub => DeviceLayout::Hub,
                    LanNodeKind::Computer(computer) => {
                        DeviceLayout::Computer(Box::new(ComputerDraft {
                            hidden: node.hidden,
                            ..computer_draft(&node.local_ip, computer)
                        }))
                    }
                },
            })
            .collect(),
    }
}

/// The global economy of `snapshot`, with names in place of the game's database ids.
pub fn economy(snapshot: &NetworkSnapshot) -> EconomyLayout {
    EconomyLayout {
        banks: snapshot
            .banks
            .iter()
            .map(|bank| BankLayout {
                number: bank.number.clone(),
                password: bank.password.clone(),
                money: bank.money,
            })
            .collect(),
        // The game keeps the payments of earlier tests' bank accounts; the copy has only the accounts of the snapshot.
        subscriptions: snapshot
            .subscriptions
            .iter()
            .filter(|payment| {
                snapshot
                    .banks
                    .iter()
                    .any(|bank| bank.number == payment.bank)
            })
            .map(|payment| BankSubscription {
                bank: payment.bank.clone(),
                kind: match payment.kind.as_ref() {
                    "BANK" => SubscriptionKind::Bank,
                    "RENTED" => SubscriptionKind::Rented,
                    "ADMIN" => SubscriptionKind::Admin,
                    other => panic!("the game's payment kind {other}"),
                },
                amount: payment.amount,
                next_payment_at: greyhack_behaviour::parse_date(&payment.next_date),
                public_ips: payment.public_ips.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        coins: snapshot
            .coins
            .iter()
            .map(|coin| CoinLayout {
                name: coin.name.clone(),
                creator_user: coin.creator_user.clone(),
                creator_password: coin.creator_password.clone(),
                address: coin.address.clone(),
                cycle_hours: coin.mining_cycle,
                reward: coin.reward,
                mined_coins: coin.mined_coins,
                max_supply: coin.max_supply,
                price: coin.price,
                prices: coin
                    .prices
                    .iter()
                    .map(|price| PriceRecord {
                        price: price.price,
                        date: price.date.clone(),
                    })
                    .collect(),
            })
            .collect(),
        wallets: snapshot
            .wallets
            .iter()
            .map(|wallet| WalletLayout {
                user: wallet.user.clone(),
                password: wallet.password.clone(),
                pin: wallet.pin.clone(),
                bank: wallet.bank.clone(),
                subwallets: wallet
                    .subwallets
                    .iter()
                    .map(|subwallet| SubWalletLayout {
                        coin: subwallet.coin.clone(),
                        user: subwallet.user.clone(),
                        password: subwallet.password.clone(),
                        balance: subwallet.balance,
                        info: subwallet.info.clone(),
                        wallet_user: wallet.user.clone(),
                    })
                    .collect(),
            })
            .collect(),
        offers: snapshot
            .offers
            .iter()
            .map(|offer| OfferLayout {
                coin: offer.coin.clone(),
                wallet_user: offer.wallet_user.clone(),
                side: if offer.side == "Buy" {
                    Side::Buy
                } else {
                    Side::Sell
                },
                price: offer.price,
                amount: offer.amount,
                subwallet: offer.subwallet.clone(),
            })
            .collect(),
    }
}

fn router_layout(local_ip: &str, router: &RouterSnapshot) -> DeviceLayout {
    let (own, forwards): (Vec<&TablePortSnapshot>, Vec<&TablePortSnapshot>) = router
        .ports
        .iter()
        .partition(|port| port.local_ip == local_ip);

    DeviceLayout::Router {
        draft: RouterDraft {
            essid: router.essid.as_ref().map(ToString::to_string),
            bssid: router.bssid.as_ref().map(ToString::to_string),
            wifi_password: router.wifi_password.clone(),
            firewall_rules: router.firewall_rules.iter().map(firewall_rule).collect(),
            firewall_active: true,
            computer: Box::new(ComputerDraft {
                kind: if router.kind == "SWITCH" {
                    DeviceKind::Switch
                } else {
                    DeviceKind::Router
                },
                users: router
                    .users
                    .iter()
                    .map(|user| User {
                        groups: user.groups.iter().map(ToString::to_string).collect(),
                        ..User::new(&user.name, &user.password)
                    })
                    .collect(),
                filesystem: filesystem(&router.files),
                ..ComputerDraft::new("router", local_ip)
            }),
        },
        ports: own
            .into_iter()
            .filter_map(|port| {
                Some(Port {
                    number: port.number,
                    service: ServiceId::from_name(&port.service)?,
                    version: port_version(&port.version),
                    closed: port.closed,
                    visible: port.visible,
                })
            })
            .collect(),
        forwards: forwards
            .into_iter()
            .map(|port| ForwardLayout {
                number: port.number,
                lan_ip: port.local_ip.clone(),
                port: port.port,
            })
            .collect(),
    }
}

/// A firewall rule, each field `Any` or its value as the game prints it.
fn firewall_rule(rule: &FirewallRuleSnapshot) -> FirewallRule {
    let field = |value: &str| (value != "Any").then(|| value.to_owned());

    FirewallRule {
        allow: rule.action == "ALLOW",
        port: rule.port.parse().ok(),
        source: field(&rule.source),
        destination: field(&rule.destination),
    }
}

fn computer_draft(local_ip: &str, computer: &ComputerSnapshot) -> ComputerDraft {
    ComputerDraft {
        kind: device_kind(&computer.kind),
        is_player: computer.player,
        network_kind: NetworkKind::of_game_name(&computer.network_kind),
        users: computer
            .users
            .iter()
            .map(|user| User {
                groups: user.groups.iter().map(ToString::to_string).collect(),
                ..User::new(&user.name, &user.password)
            })
            .collect(),
        processes: computer
            .processes
            .iter()
            .enumerate()
            .map(|(index, process)| Process {
                pid: u32::try_from(index).expect("process indexes fit u32"),
                user: process.user.clone(),
                name: process.name.clone(),
                cpu: 0.0,
                ram_mb: if process.name == SAFE_MODE_TERMINAL {
                    SAFE_MODE_TERMINAL_RAM_MB
                } else {
                    0.0
                },
                protected: process.name == "Xorg"
                    || process.name == "kernel_task"
                    || process.name == SAFE_MODE_TERMINAL,
                service: None,
                is_terminal: process.name == SAFE_MODE_TERMINAL,
            })
            .collect(),
        saved_networks: computer
            .saved_networks
            .iter()
            .map(|network| SavedNetwork {
                bssid: network.bssid.clone(),
                essid: network.essid.clone(),
                password: network.password.clone(),
                public_ip: network.public_ip.clone(),
            })
            .collect(),
        active_users: computer
            .active_users
            .iter()
            .map(ToString::to_string)
            .collect(),
        ports: computer.ports.iter().filter_map(port).collect(),
        filesystem: filesystem(&computer.files),
        hardware: hardware(computer),
        smart: computer.smart.as_ref().map(|smart| SmartApplianceSettings {
            model: smart.model.clone(),
            required_power: smart.required_power,
            required_temperature: smart.required_temperature,
            power: smart.power,
            temperature: smart.temperature,
            alarm: smart.alarm,
            override_enabled: smart.override_enabled,
            recovery_at: smart
                .broken_until
                .as_deref()
                .map_or(0, greyhack_behaviour::parse_date),
        }),
        ..ComputerDraft::new(&computer.name, local_ip)
    }
}

/// Greybel's standard hardware with the computer's network cards, the first of the kind it is connected with active.
fn hardware(computer: &ComputerSnapshot) -> Hardware {
    let active_wifi = match &*computer.active_net_card {
        "WIFI" => Some(true),
        "ETHERNET" => Some(false),
        _ => None,
    };
    let active = computer
        .net_cards
        .iter()
        .position(|card| Some(card.wifi) == active_wifi);
    let cards = computer
        .net_cards
        .iter()
        .enumerate()
        .map(|(index, card)| NetCard {
            part: Part::new(&card.name),
            monitor_enabled: card.monitor_enabled,
            active: Some(index) == active,
            ..NetCard::new(card.wifi, card.monitor_support)
        })
        .collect();

    Hardware::standard(2048, 10_000.0, &MinerHardware::default(), cards)
}

/// The kind of a computer as the game names it (`NetworkLan.TypeDevice`).
fn device_kind(name: &str) -> DeviceKind {
    match name {
        "CAMERA" => DeviceKind::Camera,
        "SMARTPHONE" => DeviceKind::Smartphone,
        "FRIDGE" => DeviceKind::Fridge,
        "MICROWAVE" => DeviceKind::Microwave,
        _ => DeviceKind::Computer,
    }
}

fn port(port: &PortSnapshot) -> Option<Port> {
    Some(Port {
        number: port.number,
        service: ServiceId::from_name(&port.service)?,
        version: port_version(&port.version),
        closed: port.closed,
        visible: port.visible,
    })
}

/// The version of a port. The game has none while the service library is missing or invalid
/// (`Router.GetPortVersion` answers `unknown`), and the port stays in its tables; the copy gives it the first version,
/// which no script reads, as the port's own version is that of the library file.
fn port_version(version: &str) -> String {
    if version == "unknown" {
        FIRST_VERSION.to_owned()
    } else {
        version.to_owned()
    }
}

/// The file system of `files`, each after the folder it is in.
fn filesystem(files: &[FileSnapshot]) -> FileSystemDraft {
    let mut filesystem = FileSystemDraft::new();

    for file in files {
        let (folder, name) = file.path.rsplit_once('/').unwrap_or(("", &file.path));
        let folder = if folder.is_empty() { "/" } else { folder };
        let (Some(parent), Some(permissions)) = (
            filesystem.folder(folder, false),
            PermissionSet::parse(&file.permissions),
        ) else {
            continue;
        };
        let mut node = if file.folder {
            NodeDraft::folder(name, &file.owner, permissions)
        } else {
            NodeDraft::file(name, &file.owner, permissions, file_data(file))
        };

        node.group.clone_from(&file.group);
        node.symlink.clone_from(&file.symlink);
        node.timestamp = file.timestamp;
        node.library = (FileKind::of_layout(file.type_file, file.binary) == FileKind::Library)
            .then(|| library(&file.content))
            .flatten()
            .map(Box::new);
        filesystem.add(parent, node);
    }

    // The snapshot's dates are the game's, after its template shift.
    filesystem.with_absolute_dates()
}

fn file_data(file: &FileSnapshot) -> FileData {
    let kind = FileKind::of_layout(file.type_file, file.binary);

    FileData {
        kind,
        content: file.content.clone(),
        command: (kind == FileKind::Binary)
            .then(|| Command::of_name(&file.command))
            .flatten(),
        size: file.size,
        ..FileData::default()
    }
}

/// Registers the game's newest version of every library (`VersionsControl.lastLibs`), so a copied library that the
/// game already replaced is not the newest one in the copy either.
pub fn register_latest_libraries(world: &World, snapshot: &NetworkSnapshot) {
    for content in &snapshot.latest_libraries {
        if let Some(library) = library(content) {
            greybel_test_support::future::block_on(
                world.modules.libraries.updates.register_library(&library),
            )
            .expect("the game's newest library is registered");
        }
    }
}

/// A library from the game's own serialized `Library` (`Library`: `listaZonaMem`, `Vulnerabilidad` and
/// `HelperHackResult`).
pub fn library(content: &str) -> Option<Library> {
    let game = serde_json::from_str::<GameLibrary>(content).ok()?;

    Some(Library {
        kind: LibraryKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.code() == game.id_lib)?,
        version: game.version.to_string(),
        zones: game
            .zones
            .into_iter()
            .map(|zone| {
                Some(MemoryZone {
                    address: zone.address,
                    hide: zone.hide,
                    weaknesses: zone
                        .weaknesses
                        .into_iter()
                        .map(|weakness| {
                            let requirements = weakness
                                .requirements
                                .iter()
                                .filter_map(|code| {
                                    Requirement::ALL
                                        .iter()
                                        .copied()
                                        .find(|requirement| requirement.code() == *code)
                                })
                                .collect::<Vec<_>>();
                            let required_lib = requirements
                                .contains(&Requirement::Library)
                                .then(|| {
                                    LibraryKind::ALL
                                        .iter()
                                        .copied()
                                        .find(|kind| kind.code() == weakness.required_lib)
                                })
                                .flatten();

                            Some(Weakness {
                                remote: weakness.remote,
                                unsec_value: weakness.unsec_value,
                                details: weakness.details,
                                requirements,
                                reward: Reward::ALL
                                    .iter()
                                    .copied()
                                    .find(|reward| reward.code() == weakness.result.reward)?,
                                user: weakness.result.user,
                                required_lib,
                                req_lib_version: weakness.req_lib_version.unwrap_or_default(),
                                metaxploit_version: weakness.metaxploit_version.to_string(),
                                num_register_users: weakness.result.num_register_users,
                                num_port_forward: weakness.result.num_port_forward,
                                num_conn_gateway: weakness.result.num_conn_gateway,
                                path_exist: weakness.result.path_exist,
                                random_path: weakness.result.random_path,
                            })
                        })
                        .collect::<Option<Vec<_>>>()?,
                })
            })
            .collect::<Option<Vec<_>>>()?,
    })
}

#[derive(Deserialize)]
struct GameLibrary {
    #[serde(rename = "idLib")]
    id_lib: i64,
    version: GameVersion,
    #[serde(rename = "listaZonaMem", deserialize_with = "game_zones")]
    zones: Vec<GameZone>,
}

#[derive(Deserialize)]
struct GameVersion {
    version: [u32; 3],
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.version[0], self.version[1], self.version[2]
        )
    }
}

#[derive(Deserialize)]
struct GameZone {
    address: String,
    hide: bool,
    #[serde(rename = "vulnerabs")]
    weaknesses: Vec<GameWeakness>,
}

#[derive(Deserialize)]
struct GameWeakness {
    #[serde(rename = "isRemote")]
    remote: bool,
    #[serde(rename = "unsecValue")]
    unsec_value: String,
    details: String,
    #[serde(rename = "requiredActions")]
    requirements: Vec<i64>,
    #[serde(rename = "helperHackResult")]
    result: GameResult,
    #[serde(rename = "requiredLib")]
    required_lib: i64,
    #[serde(rename = "reqLibVersion")]
    req_lib_version: Option<String>,
    #[serde(rename = "metaxploitVersion")]
    metaxploit_version: GameVersion,
}

#[derive(Deserialize)]
struct GameResult {
    #[serde(rename = "hackResult")]
    reward: i64,
    user: String,
    #[serde(rename = "numRegisterUsers")]
    num_register_users: u32,
    #[serde(rename = "numPortForward")]
    num_port_forward: u32,
    #[serde(rename = "numConnGateway")]
    num_conn_gateway: u32,
    #[serde(rename = "pathExist")]
    path_exist: String,
    #[serde(rename = "randomPath")]
    random_path: String,
}

/// Deserializes the dictionary without sorting its addresses: the game scans them in insertion order.
fn game_zones<'de, D>(deserializer: D) -> Result<Vec<GameZone>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ZoneVisitor;

    impl<'de> serde::de::Visitor<'de> for ZoneVisitor {
        type Value = Vec<GameZone>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("the game's memory-zone dictionary")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut zones = Vec::new();

            while let Some((_, zone)) = map.next_entry::<String, GameZone>()? {
                zones.push(zone);
            }

            Ok(zones)
        }
    }

    deserializer.deserialize_map(ZoneVisitor)
}

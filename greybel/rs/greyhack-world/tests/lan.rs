use greyhack_behaviour::{DeviceKind, is_lan_ip, same_subnet};
use greyhack_world::ComputerId;

use crate::support::lan::world;

/// Generated networks have the game's devices: LAN routers, switches and hubs under the main router, each router
/// and switch with a LAN address, a hub with none.
#[tokio::test]
async fn should_generate_lan_routers_switches_and_hubs() {
    let (world, mains) = world().await;
    let mut kinds = std::collections::HashSet::new();

    for main in &mains {
        assert!(
            main.local_ip == "192.168.0.1" || main.local_ip == "192.168.1.1",
            "{}",
            main.local_ip
        );

        let lan = world.lan(main.network).await.unwrap().unwrap();

        for (id, node) in lan.nodes() {
            if !node.is_device() || id == lan.root() {
                continue;
            }

            kinds.insert(node.kind);

            if node.kind == DeviceKind::Hub {
                assert_eq!(node.local_ip, "");
            } else {
                assert!(is_lan_ip(&node.local_ip), "{}", node.local_ip);
            }
        }
    }

    assert!(kinds.contains(&DeviceKind::Router));
    assert!(kinds.contains(&DeviceKind::Switch));
    assert!(kinds.contains(&DeviceKind::Hub));
}

/// Every computer uses a router or switch of its network, or a computer gateway of a private subnet, as its
/// gateway, in the gateway's subnet; the main router forwards every port a network is generated with.
#[tokio::test]
async fn should_give_computers_gateways_of_their_subnet() {
    let (world, mains) = world().await;

    for main in &mains {
        let lan = world.lan(main.network).await.unwrap().unwrap();

        for (_, node) in lan.nodes() {
            if node.is_device() || node.gateway == "0.0.0.0" {
                continue;
            }

            if node.gateway.is_empty() {
                // A hub's computer no router adopted keeps no gateway, as in the game.
                continue;
            }

            let gateway = lan.find(&node.gateway).expect("the gateway is in the LAN");

            assert!(
                same_subnet(&node.local_ip, &node.gateway),
                "{} uses {}",
                node.local_ip,
                lan.node(gateway).local_ip
            );
        }

        for router in world.routers_in(main.network).await.unwrap() {
            let table = world.port_table(router.id).await.unwrap();
            let own = table.first().expect("the configuration page");

            assert_eq!(
                (own.number, own.port.closed, own.port.visible),
                (8080, true, false)
            );

            if !router.is_main() {
                assert_eq!(table.len(), 1, "a LAN router forwards nothing");
            }
        }
    }
}

/// Only LAN routers and switches get firewalls, each denying everything but what it allows.
#[tokio::test]
async fn should_firewall_only_lan_routers_and_switches() {
    let (world, mains) = world().await;
    let mut firewalled = 0;

    for main in &mains {
        for router in world.routers_in(main.network).await.unwrap() {
            let rules = world.firewall_rules(router.id).await.unwrap();

            if router.is_main() {
                assert!(rules.is_empty());
                continue;
            }

            if rules.iter().any(|rule| !rule.allow) {
                firewalled += 1;
                assert!(!rules[0].allow, "the DENY rule comes first");
                assert!(rules.iter().skip(1).all(|rule| rule.allow));
            }
        }
    }

    assert!(firewalled > 0);
}

/// A computer behind a LAN router answers a ping from anywhere in the network; one behind a switch only from its
/// own subnet.
#[tokio::test]
async fn should_ping_across_lan_routers_and_only_within_a_switch() {
    let (mut world, mains) = world().await;
    let mut checked = (false, false);

    for main in &mains {
        let lan = world.lan(main.network).await.unwrap().unwrap();
        let first = lan
            .nodes()
            .find(|(_, node)| !node.is_device() && node.gateway == main.local_ip)
            .map(|(_, node)| node.key);
        let Some(from) = first else {
            continue;
        };
        let targets: Vec<(ComputerId, String, bool)> = lan
            .nodes()
            .filter(|(_, node)| !node.is_device() && !node.hidden)
            .filter_map(|(_, node)| {
                let gateway = lan.find(&node.gateway)?;
                let kind = lan.node(gateway).kind;

                (gateway != lan.root() && matches!(kind, DeviceKind::Router | DeviceKind::Switch))
                    .then(|| (node.key, node.local_ip.clone(), kind == DeviceKind::Router))
            })
            .collect();

        for (_, address, behind_router) in targets {
            let answers = world.ping(from, &address).await.unwrap();

            assert_eq!(answers, behind_router, "{address}");

            if behind_router {
                checked.0 = true;
            } else {
                checked.1 = true;
            }
        }
    }

    assert_eq!(checked, (true, true));
}

/// A script holding the shell of another computer than the player's has network on the player's computer only while
/// that computer and the devices it is connected through are available (`Computer.IsNetworkActive` with the
/// interpreter, `NetworkLan.ReversePing`).
#[tokio::test]
async fn should_need_the_devices_up_to_the_scripts_computer_for_network() {
    let (mut world, mains) = world().await;
    let player = world.player();
    let home = world.computer(player).await.unwrap().unwrap().network;
    let main = mains
        .iter()
        .find(|main| main.network != home)
        .expect("a network other than the player's");
    let lan = world.lan(main.network).await.unwrap().unwrap();
    let remote = lan
        .nodes()
        .find(|(_, node)| node.kind == DeviceKind::Computer)
        .map(|(_, node)| node.key)
        .expect("the network has a computer");

    let before = world.is_network_active(player, &[remote]).await.unwrap();

    world.reboot(main.id.computer(), false).await.unwrap();

    let after = world.is_network_active(player, &[remote]).await.unwrap();
    let holding_none = world.is_network_active(player, &[]).await.unwrap();

    assert!(before);
    assert!(!after);
    assert!(holding_none);
}

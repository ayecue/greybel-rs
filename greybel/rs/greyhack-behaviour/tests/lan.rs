use greyhack_behaviour::{FirewallRule, LanNodeId};

use crate::support::lan::{deny_all, fixture};

#[test]
fn should_put_a_devices_computers_before_its_hubs_and_its_routers() {
    let fixture = fixture();
    let lan = &fixture.lan;
    let order: Vec<&str> = lan
        .search_order()
        .into_iter()
        .map(|id| lan.node(id).key)
        .collect();

    assert_eq!(
        order,
        [
            "main",
            "pc2",
            "player",
            "switch",
            "switch pc",
            "lan router",
            "pc7",
            "hub",
            "hub pc"
        ]
    );
}

#[test]
fn should_find_nodes_by_address_but_never_a_hub() {
    let fixture = fixture();
    let lan = &fixture.lan;
    let found = (
        lan.find("172.16.3.1"),
        lan.find("172.16.3.9").map(|id| lan.node(id).key),
        lan.find(""),
        lan.find("192.168.1.99"),
    );

    assert_eq!(
        found,
        (Some(fixture.lan_router), Some("hub pc"), None, None)
    );
}

#[test]
fn should_ping_as_the_game_does() {
    let lan = fixture().lan;
    let pings = [
        // a computer whose gateway is a router answers from anywhere in the LAN
        lan.ping("192.168.1.6", "172.16.3.7"),
        // so does every device
        lan.ping("172.16.3.7", "10.0.4.1"),
        // a computer behind a switch only from its own subnet
        lan.ping("192.168.1.6", "10.0.4.2"),
        lan.ping("10.0.4.1", "10.0.4.2"),
        // public addresses always answer here, unknown LAN ones never
        lan.ping("192.168.1.6", "8.8.8.8"),
        lan.ping("192.168.1.6", "192.168.1.99"),
        // the source must be in the LAN
        lan.ping("192.168.1.99", "192.168.1.2"),
    ];

    assert_eq!(pings, [true, true, false, true, true, false, false]);
}

#[test]
fn should_not_reach_a_hidden_computer() {
    let mut lan = fixture().lan;
    let phone = lan.find("192.168.1.2").expect("pc2");

    lan.node_mut(phone).hidden = true;

    assert!(!lan.ping("192.168.1.6", "192.168.1.2"));
}

#[test]
fn should_list_a_routers_computers_then_every_device() {
    let lan = fixture().lan;
    let lists = (
        lan.router_lan("192.168.1.1", true),
        lan.router_lan("172.16.3.1", true),
    );

    assert_eq!(
        lists,
        (
            vec![
                "192.168.1.2".to_owned(),
                "192.168.1.6".to_owned(),
                "192.168.1.1".to_owned(),
                "10.0.4.1".to_owned(),
                "172.16.3.1".to_owned(),
            ],
            vec![
                "172.16.3.7".to_owned(),
                "172.16.3.9".to_owned(),
                "192.168.1.1".to_owned(),
                "10.0.4.1".to_owned(),
                "172.16.3.1".to_owned(),
            ]
        )
    );
}

#[test]
fn should_translate_through_the_port_table_of_the_router_reached() {
    let lan = fixture().lan;
    let addresses = [
        // from outside: the main router's forward
        lan.local_addresses("8.8.8.8", "45.45.45.45", 32871),
        // the LAN router's address: its own table
        lan.local_addresses("192.168.1.6", "172.16.3.1", 8080),
        lan.local_addresses("192.168.1.6", "172.16.3.1", 32871),
        // a computer's address: none
        lan.local_addresses("192.168.1.6", "172.16.3.7", 32871),
    ];

    assert_eq!(
        addresses,
        [
            ("192.168.1.1".to_owned(), "172.16.3.7".to_owned()),
            ("192.168.1.6".to_owned(), "172.16.3.1".to_owned()),
            ("192.168.1.6".to_owned(), "172.16.3.1".to_owned()),
            ("192.168.1.6".to_owned(), "172.16.3.7".to_owned()),
        ]
    );
}

#[test]
fn should_take_the_path_from_the_shared_device_down_to_the_destination() {
    let fixture = fixture();
    let lan = &fixture.lan;
    let player = lan.find("192.168.1.6").expect("player");
    let hub_pc = lan.find("172.16.3.9").expect("hub pc");
    let keys = |path: Vec<LanNodeId>| {
        path.into_iter()
            .map(|id| lan.node(id).key)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        (
            keys(lan.path(player, "172.16.3.9")),
            keys(lan.path(hub_pc, "192.168.1.2")),
            keys(lan.path(player, "1.2.3.4")),
        ),
        (vec!["hub", "lan router", "main"], vec!["main"], Vec::new())
    );
}

#[test]
fn should_block_where_a_deny_rule_matches_and_no_allow_rule_does() {
    let mut fixture = fixture();
    let allow_player = FirewallRule {
        allow: true,
        source: Some("192.168.1.6".to_owned()),
        ..deny_all()
    };

    fixture.lan.node_mut(fixture.lan_router).firewall = vec![deny_all(), allow_player];
    fixture.lan.node_mut(fixture.switch).firewall = vec![deny_all()];

    let lan = &fixture.lan;
    let blocked = [
        lan.blocks("192.168.1.6", "192.168.1.6", 22, "172.16.3.7"),
        lan.blocks("192.168.1.2", "192.168.1.2", 22, "172.16.3.7"),
        lan.blocks("192.168.1.6", "192.168.1.6", 22, "10.0.4.2"),
        // a connection inside the switch's subnet still passes the switch
        lan.blocks("10.0.4.2", "10.0.4.2", 22, "10.0.4.2"),
        lan.blocks("192.168.1.6", "192.168.1.6", 22, "192.168.1.2"),
    ];

    assert_eq!(blocked, [false, true, true, true, false]);
}

#[test]
fn should_give_the_address_after_the_highest_of_the_subnet() {
    let lan = fixture().lan;

    assert_eq!(
        (
            lan.next_address("192.168.1.1"),
            lan.next_address("172.16.3.1"),
            lan.next_address("10.0.4.1")
        ),
        (
            "192.168.1.7".to_owned(),
            "172.16.3.10".to_owned(),
            "10.0.4.3".to_owned()
        )
    );
}

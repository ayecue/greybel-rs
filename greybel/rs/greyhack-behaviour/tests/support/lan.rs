use greyhack_behaviour::{DeviceKind, FirewallRule, Lan, LanNode, LanNodeId, LanPort};

/// The test fixture's network as the game generated it, cut down: the main router 192.168.1.1 with two computers
/// and a forward to one behind a LAN router; the LAN router 172.16.3.1 with two computers; a switch 10.0.4.1 with
/// one computer; a hub with a computer the LAN router adopted.
pub struct Fixture {
    pub lan: Lan<&'static str>,
    pub lan_router: LanNodeId,
    pub switch: LanNodeId,
}

pub fn computer(key: &'static str, address: &str, gateway: &str) -> LanNode<&'static str> {
    LanNode {
        gateway: gateway.to_owned(),
        ..LanNode::new(key, DeviceKind::Computer, address)
    }
}

pub fn deny_all() -> FirewallRule {
    FirewallRule {
        allow: false,
        port: None,
        source: None,
        destination: None,
    }
}

pub fn fixture() -> Fixture {
    let mut lan = Lan::new(LanNode {
        ports: vec![LanPort {
            number: 32871,
            lan_ip: "172.16.3.7".to_owned(),
        }],
        ..LanNode::new("main", DeviceKind::Router, "192.168.1.1")
    });
    let root = lan.root();
    let lan_router = lan.add(
        root,
        LanNode {
            ports: vec![LanPort {
                number: 8080,
                lan_ip: "172.16.3.1".to_owned(),
            }],
            ..LanNode::new("lan router", DeviceKind::Router, "172.16.3.1")
        },
    );
    let switch = lan.add(root, LanNode::new("switch", DeviceKind::Switch, "10.0.4.1"));

    lan.add(root, computer("pc2", "192.168.1.2", "192.168.1.1"));
    lan.add(root, computer("player", "192.168.1.6", "192.168.1.1"));
    lan.add(lan_router, computer("pc7", "172.16.3.7", "172.16.3.1"));

    let hub = lan.add(lan_router, LanNode::new("hub", DeviceKind::Hub, ""));

    lan.add(hub, computer("hub pc", "172.16.3.9", "172.16.3.1"));
    lan.add(switch, computer("switch pc", "10.0.4.2", "10.0.4.1"));

    Fixture {
        lan,
        lan_router,
        switch,
    }
}

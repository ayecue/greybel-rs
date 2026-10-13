use greyhack_behaviour::FirewallRule;

pub fn rule(allow: bool, port: Option<u16>) -> FirewallRule {
    FirewallRule {
        allow,
        port,
        source: None,
        destination: None,
    }
}

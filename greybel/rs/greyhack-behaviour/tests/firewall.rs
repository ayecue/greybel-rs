use greyhack_behaviour::firewall_allows;

use crate::support::firewall::rule;

#[test]
fn should_block_a_port_matching_a_deny_rule() {
    let rules = vec![rule(false, Some(22))];
    let allowed = (
        firewall_allows(true, &rules, "1.1.1.1", 22, "192.168.0.2"),
        firewall_allows(true, &rules, "1.1.1.1", 80, "192.168.0.2"),
    );

    assert_eq!(allowed, (false, true));
}

#[test]
fn should_let_an_allow_rule_override_a_matching_deny_rule() {
    let rules = vec![rule(false, Some(22)), rule(true, Some(22))];

    assert!(firewall_allows(true, &rules, "1.1.1.1", 22, "192.168.0.2"));
}

#[test]
fn should_allow_connections_when_the_firewall_is_disabled() {
    let rules = vec![rule(false, None)];

    assert!(firewall_allows(false, &rules, "1.1.1.1", 22, "192.168.0.2"));
}

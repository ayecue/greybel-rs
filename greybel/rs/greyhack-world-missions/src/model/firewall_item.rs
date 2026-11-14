/// `HwFirewallRule.Action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallAction {
    Allow,
    Deny,
}

/// `HwFirewallRule.Rule.Status`: whether a rule matches anything or one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallMatch {
    Any,
    Single,
}

/// `MissionConfig.FirewallItem`: a rule the mission adds to the main router's firewall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallItem {
    pub action: FirewallAction,
    pub status_port: FirewallMatch,
    pub port: i32,
    pub status_source_ip: FirewallMatch,
    pub pc_index_source_ip: i32,
    pub status_dest_ip: FirewallMatch,
    pub pc_index_dest_ip: i32,
}

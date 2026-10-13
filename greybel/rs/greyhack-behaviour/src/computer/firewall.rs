//! Matching and evaluation of firewall rules.

/// Rule returned by `firewall_rules`; fields set to `None` are wildcards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirewallRule {
    pub allow: bool,
    pub port: Option<u16>,
    pub source: Option<String>,
    pub destination: Option<String>,
}

impl FirewallRule {
    /// Checks a connection against this rule using `HwFirewallRule` semantics.
    #[must_use]
    pub fn matches(&self, source: &str, port: u16, destination: &str) -> bool {
        self.port.is_none_or(|rule_port| rule_port == port)
            && self
                .source
                .as_deref()
                .is_none_or(|rule_source| rule_source == source)
            && self
                .destination
                .as_deref()
                .is_none_or(|rule_destination| rule_destination == destination)
    }
}

/// Evaluates `rules` when active; any matching allow rule overrides matching deny rules.
#[must_use]
pub fn firewall_allows(
    active: bool,
    rules: &[FirewallRule],
    source: &str,
    port: u16,
    destination: &str,
) -> bool {
    if !active {
        return true;
    }

    let mut denied = false;
    let mut allowed = false;

    for rule in rules {
        if rule.matches(source, port, destination) {
            if rule.allow {
                allowed = true;
            } else {
                denied = true;
            }
        }
    }

    !denied || allowed
}

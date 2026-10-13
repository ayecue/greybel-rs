use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::router_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{FirewallEntry, RouterFirewallRules};

#[native(method)]
pub fn firewall_rules(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(router) = router_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = RouterFirewallRules {
        program: program(vm)?,
        router,
    };

    Ok(respond(
        vm,
        server(vm)?.send_router_firewall_rules(&message),
        |_, rules| rules.map(|rules| rules.iter().map(firewall_text).collect::<Vec<_>>()),
    ))
}

/// Formats a rule as `ACTION port source destination`, with `Any` for wildcard fields.
fn firewall_text(rule: &FirewallEntry) -> String {
    const ANY: &str = "Any";

    let action = if rule.allow { "ALLOW" } else { "DENY" };
    let port = rule
        .port
        .map_or_else(|| ANY.to_owned(), |port| port.to_string());
    let source = rule.source.as_deref().unwrap_or(ANY);
    let destination = rule.destination.as_deref().unwrap_or(ANY);

    format!("{action} {port} {source} {destination}")
}

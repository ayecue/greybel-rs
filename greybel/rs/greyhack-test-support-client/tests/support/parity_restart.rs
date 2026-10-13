//! What the restart parity tests read of the player's computer, and the routers and programs they use.

use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};

use super::remote::script_string;

/// What a script reads of the player's connection and its processes, without their PIDs.
pub const STATE: &str = "pc = get_shell.host_computer
print [pc.active_net_card, pc.public_ip, pc.local_ip, pc.network_gateway]
names = []
for line in pc.show_procs.split(char(10))
  names.push(line.split(\" \")[-1])
end for
names.sort
print names";

/// A router with Wi-Fi: its BSSID, ESSID and password, and its LAN address.
pub struct WifiRouter {
    pub bssid: String,
    pub essid: String,
    pub password: String,
    pub local_ip: String,
}

/// The router of `snapshot`'s node `index` when it has Wi-Fi.
pub fn wifi_router(snapshot: &NetworkSnapshot, index: usize) -> Option<WifiRouter> {
    let LanNodeKind::Router(router) = &snapshot.nodes[index].node else {
        return None;
    };

    Some(WifiRouter {
        bssid: router.bssid.as_ref()?.to_string(),
        essid: router.essid.as_ref()?.to_string(),
        password: router.wifi_password.to_string(),
        local_ip: snapshot.nodes[index].local_ip.to_string(),
    })
}

/// The main router and a LAN router of `snapshot` that both have Wi-Fi.
pub fn routers(snapshot: &NetworkSnapshot) -> Option<(WifiRouter, WifiRouter)> {
    let lan = (1..snapshot.nodes.len()).find_map(|index| wifi_router(snapshot, index))?;

    Some((wifi_router(snapshot, 0)?, lan))
}

/// A script that connects to the network of `router` and then reads the state.
pub fn connect(router: &WifiRouter) -> String {
    format!(
        "print get_shell.host_computer.connect_wifi(\"wlan0\", {}, {}, {})\n{STATE}",
        script_string(&router.bssid),
        script_string(&router.essid),
        script_string(&router.password),
    )
}

/// What the safe-mode terminal does that a terminal doesn't: `other` has a program in its home that others can't
/// execute, and one in a folder others can't enter (`ShellIntrinsics.launch`, `GreyInterpreter.isSafeTerminal`). The
/// programs are built where they are, so that no copy through a download of the client is involved.
pub const SETUP: &str = "root = get_shell(\"root\", \"test\")
pc = root.host_computer
print pc.create_user(\"other\", \"pw\")
print pc.create_folder(\"/home/other\", \"locked\")
pc.touch(home_dir, \"child.src\")
pc.File(home_dir + \"/child.src\").set_content(\"print(\"\"ran\"\")\")
print \"build=\" + root.build(home_dir + \"/child.src\", \"/home/other\")
print \"build=\" + root.build(home_dir + \"/child.src\", \"/home/other/locked\")
print pc.File(\"/home/other/child\").chmod(\"o-x\", 0)
print pc.File(\"/home/other/locked\").chmod(\"o-x\", 0)
print pc.File(\"/home/other/locked/child\").chmod(\"o-x\", 0)";

pub const LAUNCH: &str = "sh = get_shell(\"other\", \"pw\")
print sh.launch(\"/home/other/child\")
print sh.launch(\"/home/other/locked/child\")
print sh.launch(\"/home/other/missing\")";

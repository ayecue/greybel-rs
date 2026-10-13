//! Programs and libraries the hack shop, the computer shop, and the net services sell
//! (`Router.PostGenComputer`, `PlayerUtils.UpdateHackshopLibs`).

use crate::{LibraryKind, NetworkKind};

/// Programs `(name, price)` a hack shop sells (`Router.PostGenComputer`).
const HACK_SHOP_PROGRAMS: &[(&str, u32)] = &[
    ("AdminMonitor.exe", 15),
    ("decipher", 0),
    ("scanlib", 45),
    ("scanrouter", 0),
    ("sniffer", 25),
    ("rshell-server", 50),
    ("rshell-interface", 85),
];
/// Programs `(name, price)` a computer shop sells (`Router.PostGenComputer`).
const COMPUTER_SHOP_PROGRAMS: &[(&str, u32)] = &[
    ("nmap", 0),
    ("smtp-user-list", 0),
    ("ssh-server", 0),
    ("http-server", 0),
    ("ftp-server", 0),
    ("repository-server", 0),
    ("wallet", 0),
    ("Stocks.exe", 0),
    ("News.exe", 0),
    ("ExploitReport.exe", 0),
    ("HiringManager.exe", 0),
    ("ConfigCompany.exe", 0),
];
/// Programs `(name, price)` the net services sell (`Router.PostGenComputer`).
const NET_SERVICES_PROGRAMS: &[(&str, u32)] = &[("ConfigLan.exe", 0)];

/// The libraries a hack shop sells (`PlayerUtils.UpdateHackshopLibs`).
pub const HACK_SHOP_LIBRARIES: [LibraryKind; 3] = [
    LibraryKind::Metaxploit,
    LibraryKind::Crypto,
    LibraryKind::Rshell,
];

/// Returns the programs `(name, price)` the shops of a network of `kind` list on their HTTP service,
/// or `None` for networks without a shop (`Router.PostGenComputer`).
#[must_use]
pub const fn shop_programs(kind: NetworkKind) -> Option<&'static [(&'static str, u32)]> {
    match kind {
        NetworkKind::HackShop => Some(HACK_SHOP_PROGRAMS),
        NetworkKind::ComputerShop => Some(COMPUTER_SHOP_PROGRAMS),
        NetworkKind::NetServices => Some(NET_SERVICES_PROGRAMS),
        _ => None,
    }
}

/// Returns the key of the English detail text of the program or library file `name`: its name
/// without the `.exe` or `.so` extension, in capitals, and `_DETAIL` (`Router.PostGenComputer`,
/// `TranslationManager.GetText`).
///
/// The game strips the extension in two orders, before capitals for hack shop programs and the
/// repository listing and after them for computer shop programs; they agree for every name the
/// shops sell, whose extension is lowercase `.exe`.
#[must_use]
pub fn detail_key(name: &str) -> String {
    format!(
        "{}_DETAIL",
        name.replace(".exe", "").replace(".so", "").to_uppercase()
    )
}

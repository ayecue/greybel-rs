//! IP address validation and subnet checks for Grey Hack.

/// Address used by a computer with no network.
pub const NO_ADDRESS: &str = "0.0.0.0";

const PARTS: usize = 4;

/// Checks for four octets from 0 to 255 without leading zeros, excluding `0.0.0.0`.
#[must_use]
pub fn is_valid_ip(address: &str) -> bool {
    if address == NO_ADDRESS {
        return false;
    }

    let parts: Vec<&str> = address.split('.').collect();

    parts.len() == PARTS
        && parts.iter().all(|part| {
            part.parse::<i32>()
                .is_ok_and(|value| (0..=255).contains(&value) && value.to_string() == *part)
        })
}

/// First two octets of the subnets LANs use (`NetworkLan.GlobalLan.localRanges`).
pub const LOCAL_RANGES: [&str; 3] = ["10.0", "172.16", "192.168"];

/// Checks whether `address` is valid and in `10.x`, `172.16` through `172.31`, or `192.168`
/// This matches `IP.IsLanIp`.
#[must_use]
pub fn is_lan_ip(address: &str) -> bool {
    if !is_valid_ip(address) {
        return false;
    }

    let parts: Vec<u32> = address
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();

    matches!(parts[..], [10, ..] | [172, 16..=31, ..] | [192, 168, ..])
}

/// Checks whether `address` belongs to a public range unused by networks (`IP.IsReserved`).
#[must_use]
pub fn is_reserved_ip(address: &str) -> bool {
    let Some(address) = address_number(address) else {
        return false;
    };

    greyhack_world_assets::reserved_address_ranges()
        .iter()
        .any(|(first, last)| {
            address_number(first).is_some_and(|first| first <= address)
                && address_number(last).is_some_and(|last| address <= last)
        })
}

/// Checks whether `address` is valid and outside the reserved ranges, as `ServerMap.GetRouter`
/// requires of a public address (`IP.IsValidIP` with `checkReserved`).
#[must_use]
pub fn is_valid_unreserved_ip(address: &str) -> bool {
    is_valid_ip(address) && !is_reserved_ip(address)
}

/// Checks whether `address` is a valid public address: valid, outside the reserved ranges, and not
/// a LAN address (`PanelMainGuild.OnHireCrownShield`).
#[must_use]
pub fn is_public_ip(address: &str) -> bool {
    is_valid_unreserved_ip(address) && !is_lan_ip(address)
}

/// Ranges no generated public address uses (`IPGroupManager.InitializeReservedRanges`). Unlike
/// [`is_reserved_ip`]'s game data, they include `192.168.0.0/16`.
const GROUP_RESERVED_RANGES: [(u32, u32); 16] = [
    (0x0000_0000, 0x00FF_FFFF),
    (0x0A00_0000, 0x0AFF_FFFF),
    (0x6440_0000, 0x647F_FFFF),
    (0x7F00_0000, 0x7FFF_FFFF),
    (0xA9FE_0000, 0xA9FE_FFFF),
    (0xAC10_0000, 0xAC1F_FFFF),
    (0xC000_0000, 0xC000_00FF),
    (0xC000_0200, 0xC000_02FF),
    (0xC058_6300, 0xC058_63FF),
    (0xC0A8_0000, 0xC0A8_FFFF),
    (0xC612_0000, 0xC613_FFFF),
    (0xC633_6400, 0xC633_64FF),
    (0xCB00_7100, 0xCB00_71FF),
    (0xE000_0000, 0xEFFF_FFFF),
    (0xF000_0000, 0xFFFF_FFFE),
    (0xFFFF_FFFF, 0xFFFF_FFFF),
];

/// Checks whether `address` is in a range no generated public address uses
/// (`IPGroupManager.IsReserved`).
#[must_use]
pub fn is_group_reserved_ip(address: &str) -> bool {
    address_number(address).is_some_and(|address| {
        GROUP_RESERVED_RANGES
            .iter()
            .any(|(first, last)| (*first..=*last).contains(&address))
    })
}

/// Converts four octets from 0 to 255 to one number, with the first octet highest; otherwise returns `None`.
fn address_number(address: &str) -> Option<u32> {
    address.parse::<std::net::Ipv4Addr>().ok().map(u32::from)
}

/// Checks whether an address is valid and belongs to a LAN range.
#[must_use]
pub fn is_valid_lan_ip(address: &str) -> bool {
    is_valid_ip(address) && is_lan_ip(address)
}

/// Returns the LAN address of the main router of 192.168 network number `network` (0 or 1)
/// (`NetworkLan.Generate`).
#[must_use]
pub fn main_router_address(network: u32) -> String {
    format!("192.168.{network}.1")
}

/// Yields hosts 2 through 254 in the subnet of the router at `router_ip`.
pub fn lan_hosts(router_ip: &str) -> impl Iterator<Item = String> + '_ {
    let subnet = router_ip
        .rsplit_once('.')
        .map_or("192.168.0", |(subnet, _)| subnet);

    (2..=254).map(move |host| format!("{subnet}.{host}"))
}

/// Checks that both addresses are valid and have the same first three octets.
#[must_use]
pub fn same_subnet(first: &str, second: &str) -> bool {
    if !is_valid_ip(first) || !is_valid_ip(second) {
        return false;
    }

    match (first.rfind('.'), second.rfind('.')) {
        (Some(a), Some(b)) => first[..a] == second[..b],
        _ => false,
    }
}

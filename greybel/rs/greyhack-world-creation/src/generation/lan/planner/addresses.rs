//! Dotted addresses as four numbers.

pub(super) fn parse(address: &str) -> [u32; 4] {
    let mut parts = [0; 4];

    for (part, text) in parts.iter_mut().zip(address.split('.')) {
        *part = text.parse().unwrap_or(0);
    }

    parts
}

/// `IP.NextIP(count)`: adds `count` to the final octet without wrapping.
pub(super) const fn next_address(mut address: [u32; 4], count: u32) -> [u32; 4] {
    address[3] += count;

    address
}

pub(super) fn format_address(address: [u32; 4]) -> String {
    format!(
        "{}.{}.{}.{}",
        address[0], address[1], address[2], address[3]
    )
}

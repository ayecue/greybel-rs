use std::sync::LazyLock;

/// Reserved public address ranges from the game's `ReservedIPs` (`IP.reservedIPs`), inclusive of each range's
/// first and last addresses.
#[must_use]
pub fn reserved_address_ranges() -> &'static [(String, String)] {
    static RANGES: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
        let ranges: serde_json::Map<String, serde_json::Value> = serde_json::from_str(
            crate::bundle::text("config/reserved_ips.json").trim_start_matches('\u{feff}'),
        )
        .expect("the bundled reserved addresses are valid");

        ranges
            .into_iter()
            .map(|(first, last)| (first, last.as_str().expect("a last address").to_owned()))
            .collect()
    });

    &RANGES
}

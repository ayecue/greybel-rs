//! Police traffic licence plates (`LicensePlateGenerator`). Each 17-character `0-9A-Z` plate contains a
//! 7-character world-seed permutation of the driver's work network address and a 10-character tag derived
//! from that address and the driver's surname.

use crate::SECONDS_PER_DAY;
use crate::base::world_seed::WorldSeed;

/// Seconds a camera stays closed to the player who started an accident: 12 hours
/// (`PlayerHelperServer.EnableCarAccidentServerRpc`).
pub const ACCIDENT_COOLDOWN_SECONDS: u64 = SECONDS_PER_DAY / 2;

const DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ENCODED_ADDRESS_LENGTH: usize = 7;
const TAG_LENGTH: usize = 10;
const PLATE_LENGTH: usize = ENCODED_ADDRESS_LENGTH + TAG_LENGTH;
const ADDRESS_LABEL: &str = "license-plate-ip-v4";
const TAG_LABEL: &str = "license-plate-tag-v4";

/// Generates a plate for an NPC with `surname` at network `address`
/// (`LicensePlateGenerator.GenerateLicensePlate`). Returns `None` unless the address has four octets from 0 to 255.
#[must_use]
pub fn license_plate(seed: &WorldSeed, address: &str, surname: &str) -> Option<String> {
    let address = address.trim();
    let surname = surname.trim().to_lowercase();
    let address_code = seed.permute_u32(ADDRESS_LABEL, address_number(address)?);
    let tag = seed.derive_u64(TAG_LABEL, &[address, &surname]);

    Some(base36(u64::from(address_code), ENCODED_ADDRESS_LENGTH) + &base36(tag, TAG_LENGTH))
}

/// Trims and uppercases `plate` when it has 17 valid characters.
/// This matches `LicensePlateGenerator.NormalizeLicensePlate`.
#[must_use]
pub fn normalize_license_plate(plate: &str) -> Option<String> {
    let plate = plate.trim().to_uppercase();

    (plate.chars().count() == PLATE_LENGTH && plate.chars().all(|digit| DIGITS.contains(digit)))
        .then_some(plate)
}

/// Decodes the driver's work network address from a normalized plate (`LicensePlateGenerator.GetEncodedIpAddress`).
#[must_use]
pub fn license_plate_address(seed: &WorldSeed, plate: &str) -> String {
    let plate_code = plate
        .chars()
        .take(ENCODED_ADDRESS_LENGTH)
        .filter_map(|digit| DIGITS.find(digit))
        .fold(0_u64, |number, digit| number * 36 + digit as u64);
    let address = seed.depermute_u32(ADDRESS_LABEL, plate_code as u32);

    address
        .to_be_bytes()
        .map(|octet| octet.to_string())
        .join(".")
}

fn address_number(address: &str) -> Option<u32> {
    address.split('.').try_fold(0_u32, |number, octet| {
        Some((number << 8) | u32::from(octet.parse::<u8>().ok()?))
    })
}

fn base36(mut number: u64, length: usize) -> String {
    let mut digits = vec![b'0'; length];

    for digit in digits.iter_mut().rev() {
        *digit = DIGITS.as_bytes()[(number % 36) as usize];
        number /= 36;
    }

    String::from_utf8(digits).expect("base 36 digits")
}

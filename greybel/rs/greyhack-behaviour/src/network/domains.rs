//! Web domains and the names addresses spell (`IpGenerator`, `Networking.GetDomainWeb`,
//! `StringExtensions.CleanDomainName`).

use mono_random::Random;

use crate::{WorldSeed, is_reserved_ip};

const CONSONANTS: &[u8] = b"bcdfghjklmnpqrstvwxyz";
const VOWELS: &[u8] = b"aeiou";
const CONSONANTS_LABEL: &str = "ip-name-consonants-v1";
const VOWELS_LABEL: &str = "ip-name-vowels-v1";

/// Prefix for map web addresses (`ServerMap.RouterNode`).
pub const WEB_PREFIX: &str = "www.";
/// Top-level domains a network can have (`Networking.GetDomainWeb`).
pub const TOP_LEVEL_DOMAINS: &[&str] = &["com", "net", "org", "info"];
/// World-seed label of the router node that draws a network's top-level domain (`ServerMap.RouterNode`).
pub const ROUTER_NODE_LABEL: &str = "router-node-v1";

/// Returns the domain of the network at `public_ip`, without `www.` (`ServerMap.RouterNode`,
/// `IpGenerator.GetDomainName`): the address encoded as a name and a top-level domain drawn from the
/// router node's seed.
#[must_use]
pub fn network_domain(seed: &WorldSeed, public_ip: &str) -> String {
    let value = public_ip.parse::<std::net::Ipv4Addr>().map_or(0, u32::from);
    let mut random = Random::new(seed.derive_int(ROUTER_NODE_LABEL, &[public_ip]));

    web_domain(&LetterTableSet::of(seed).name_of(value), &mut random)
}

/// Lowercases `name` and appends a top-level domain drawn from `random`
/// (`Networking.GetDomainWeb`).
#[must_use]
pub fn web_domain(name: &str, random: &mut Random) -> String {
    format!(
        "{}.{}",
        name.to_lowercase(),
        TOP_LEVEL_DOMAINS[random.next_below(TOP_LEVEL_DOMAINS.len())]
    )
}

/// Decodes `domain` using world letter tables (`IpGenerator.UniqueNameToIP`). Returns `None` for
/// invalid, reserved, or mismatched addresses.
#[must_use]
pub fn unique_name_to_ip(seed: &WorldSeed, domain: &str) -> Option<String> {
    let tables = LetterTableSet::of(seed);
    let name = clean_domain_name(domain);
    let mut value = 0_u32;
    let mut consonant = name.encode_utf16().count() % 2 == 1;

    for character in name.bytes() {
        let table = if consonant {
            &tables.consonants
        } else {
            &tables.vowels
        };
        let index = table.iter().position(|candidate| *candidate == character)?;

        value = value
            .wrapping_mul(table.len() as u32)
            .wrapping_add(index as u32);
        consonant = !consonant;
    }

    let address = std::net::Ipv4Addr::from(value).to_string();

    (!is_reserved_ip(&address) && tables.name_of(value) == name).then_some(address)
}

/// Removes `.org`, `.net`, `.com`, `.info`, and `www.` from `domain`
/// (`StringExtensions.CleanDomainName`).
#[must_use]
pub fn clean_domain_name(domain: &str) -> String {
    [".org", ".net", ".com", ".info", WEB_PREFIX]
        .into_iter()
        .fold(domain.to_owned(), |text, part| text.replace(part, ""))
}

/// World-shuffled consonants and vowels used to encode addresses as names (`IpGenerator`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetterTableSet {
    pub consonants: Vec<u8>,
    pub vowels: Vec<u8>,
}

impl LetterTableSet {
    /// Shuffles the letters by the world `seed`.
    #[must_use]
    pub fn of(seed: &WorldSeed) -> Self {
        Self {
            consonants: shuffle_letters(CONSONANTS, seed.derive_int(CONSONANTS_LABEL, &[])),
            vowels: shuffle_letters(VOWELS, seed.derive_int(VOWELS_LABEL, &[])),
        }
    }

    /// Encodes address `value` as a name ending in a consonant (`IpGenerator.IPToUniqueName`).
    #[must_use]
    pub fn name_of(&self, mut value: u32) -> String {
        let mut letters = Vec::new();
        let mut consonant = true;

        while value != 0 {
            let table = if consonant {
                &self.consonants
            } else {
                &self.vowels
            };

            letters.push(table[(value % table.len() as u32) as usize]);
            value /= table.len() as u32;
            consonant = !consonant;
        }

        letters.reverse();

        String::from_utf8(letters).unwrap_or_default()
    }
}

/// Sorts letters by from_seed random keys with stable ties, matching `ShuffleArray` LINQ `OrderBy`.
fn shuffle_letters(letters: &[u8], seed: i32) -> Vec<u8> {
    let mut random = Random::new(seed);
    let mut letter_keys: Vec<(i32, u8)> = letters
        .iter()
        .map(|letter| (random.next_int(), *letter))
        .collect();

    letter_keys.sort_by_key(|(key, _)| *key);

    letter_keys.into_iter().map(|(_, letter)| letter).collect()
}

use greyhack_behaviour::{LetterTableSet, network_domain, unique_name_to_ip};

use crate::support::domains::seed;

#[test]
fn should_resolve_the_name_an_address_spells() {
    let seed = seed(7);
    let name = LetterTableSet::of(&seed).name_of(0x5A0B_0C0D);
    let address = unique_name_to_ip(&seed, &format!("www.{name}.com"));

    assert_eq!(address.as_deref(), Some("90.11.12.13"));
}

#[test]
fn should_refuse_a_name_that_spells_another_address() {
    let seed = seed(7);
    // A leading letter whose index is 0 adds nothing to the address, so the address names something shorter.
    let tables = LetterTableSet::of(&seed);
    let name = format!(
        "{}{}",
        tables.consonants[0] as char,
        tables.name_of(0x5A0B_0C0D)
    );

    assert_eq!(unique_name_to_ip(&seed, &format!("www.{name}.com")), None);
}

#[test]
fn should_refuse_a_domain_whose_letters_do_not_alternate() {
    let address = unique_name_to_ip(&seed(7), "www.nowhere.org");

    assert_eq!(address, None);
}

#[test]
fn should_shuffle_the_letters_by_the_world_seed() {
    let (one, other) = (LetterTableSet::of(&seed(1)), LetterTableSet::of(&seed(2)));

    assert_ne!(
        (one.consonants, one.vowels),
        (other.consonants, other.vowels)
    );
}

/// A generated network's domain spells its address (`ServerMap.RouterNode`, `IpGenerator.GetDomainName`), so
/// resolving it gives the address back; the top-level domain comes from the router node's seed.
#[test]
fn should_name_a_network_by_its_address() {
    let seed = seed(7);
    let domain = network_domain(&seed, "90.11.12.13");
    let (name, top_level) = domain.split_once('.').unwrap();

    assert_eq!(name, LetterTableSet::of(&seed).name_of(0x5A0B_0C0D));
    assert!(["com", "net", "org", "info"].contains(&top_level));
    assert_eq!(network_domain(&seed, "90.11.12.13"), domain);
    assert_eq!(
        unique_name_to_ip(&seed, &format!("www.{domain}")).as_deref(),
        Some("90.11.12.13")
    );
}

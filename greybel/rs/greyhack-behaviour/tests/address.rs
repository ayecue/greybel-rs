use greyhack_behaviour::{
    is_group_reserved_ip, is_lan_ip, is_public_ip, is_reserved_ip, is_valid_ip,
    is_valid_unreserved_ip,
};

#[test]
fn should_validate_addresses_like_grey_hack() {
    let addresses = [
        "192.168.0.2",
        "0.0.0.0",
        "1.2.3",
        "1.2.3.256",
        "1.2.3.04",
        "1.2.3.+4",
    ];
    let [valid, zeros, short, too_large, leading_zero, signed] = addresses.map(is_valid_ip);

    assert!(valid);
    assert!(!zeros);
    assert!(!short);
    assert!(!too_large);
    assert!(!leading_zero);
    assert!(!signed);
}

#[test]
fn should_recognize_lan_addresses_like_grey_hack() {
    let addresses = [
        "10.0.5.1",
        "192.168.1.1",
        "192.169.1.1",
        "10.1.0.1",
        "172.31.0.1",
        "172.32.0.1",
        "172.16.x.y",
    ];
    let [
        ten,
        one_nine_two,
        public,
        ten_one,
        last_172,
        past_172,
        invalid,
    ] = addresses.map(is_lan_ip);

    assert!(ten);
    assert!(one_nine_two);
    assert!(!public);
    assert!(ten_one);
    assert!(last_172);
    assert!(!past_172);
    assert!(!invalid);
}

/// Generated public addresses avoid `IPGroupManager`'s ranges, which add `192.168.0.0/16` to the game's
/// reserved-address data.
#[test]
fn should_reserve_the_address_group_ranges() {
    assert!(is_group_reserved_ip("192.168.4.4"));
    assert!(!is_reserved_ip("192.168.4.4"));

    for address in [
        "0.1.2.3",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.1.1",
        "172.20.0.1",
        "198.18.0.1",
        "224.0.0.1",
        "255.255.255.255",
    ] {
        assert!(is_group_reserved_ip(address), "{address}");
    }

    for address in [
        "1.1.1.1",
        "100.128.0.1",
        "172.32.0.1",
        "192.169.0.1",
        "223.255.255.255",
    ] {
        assert!(!is_group_reserved_ip(address), "{address}");
    }
}

#[test]
fn should_accept_a_public_address_only_when_valid_unreserved_and_not_lan() {
    let reserved = greyhack_world_assets::reserved_address_ranges()[0]
        .0
        .clone();

    assert!(is_public_ip("8.8.8.8"));
    assert!(!is_public_ip("192.168.0.1"));
    assert!(!is_public_ip("1.2.3"));
    assert!(!is_public_ip(&reserved));
    assert!(is_valid_unreserved_ip("192.168.0.1"));
    assert!(!is_valid_unreserved_ip(&reserved));
}

use greyhack_behaviour::{
    WorldSeed, license_plate, license_plate_address, normalize_license_plate,
};

#[test]
fn should_make_a_plate_whose_address_reads_back() {
    let seed = WorldSeed::new(vec![7; 32]);

    let plate = license_plate(&seed, "1.2.3.4", "Smith").unwrap();

    assert_eq!(plate.len(), 17);
    assert_eq!(
        normalize_license_plate(&format!(" {} ", plate.to_lowercase())),
        Some(plate.clone())
    );
    assert_eq!(license_plate_address(&seed, &plate), "1.2.3.4");
}

#[test]
fn should_tell_drivers_of_one_network_apart_by_their_surname() {
    let seed = WorldSeed::new(vec![7; 32]);

    let [smith, jones, other] = [
        ("1.2.3.4", "Smith"),
        ("1.2.3.4", "Jones"),
        ("1.2.3.5", "Smith"),
    ]
    .map(|(address, surname)| license_plate(&seed, address, surname).unwrap());

    assert_eq!(smith[..7], jones[..7]);
    assert_ne!(smith[7..], jones[7..]);
    assert_ne!(smith[..7], other[..7]);
}

#[test]
fn should_reject_what_isnt_a_plate() {
    let texts = [
        "not-a-plate",
        "ABC",
        "0123456789ABCDEFGH",
        "0123456789ABCDEF!",
    ];

    let normalized = texts.map(normalize_license_plate);

    assert_eq!(normalized, [None, None, None, None]);
}

/// Plates the game's `LicensePlateGenerator` makes for a secret of 32 bytes of 7.
#[test]
fn should_make_the_plates_the_game_makes() {
    let seed = WorldSeed::new(vec![7; 32]);

    let plates = [("1.2.3.4", "Smith"), ("203.0.113.77", "Ölmez")]
        .map(|(address, surname)| license_plate(&seed, address, surname).unwrap());

    assert_eq!(plates, ["0R2N9KCAVWE0J37G4", "1DYJM2UDU72F8ZDAY"]);
}

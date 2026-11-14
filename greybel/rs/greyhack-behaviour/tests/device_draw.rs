use greyhack_behaviour::{DeviceDraw, DeviceKind, NetworkKind, Rng};

/// The loot table is drawn once per device, also when the computer is its only entry (`RDSTable.rdsResult`).
#[test]
fn should_draw_once_per_device_in_a_residential_network() {
    let mut draw = DeviceDraw::default();
    let mut rng = Rng::new(7);
    let mut skipped = rng.clone();

    for _ in 0..20 {
        assert_eq!(
            draw.draw(&mut rng, NetworkKind::Residential),
            DeviceKind::Computer
        );
        skipped.skip(1);
    }

    assert_eq!(rng, skipped);
}

/// The fridge and the microwave count as taken as soon as they are candidates, drawn or not
/// (`NetworkLan.GetRandomTypeDevice`).
#[test]
fn should_offer_the_fridge_and_the_microwave_once() {
    for seed in 0..50 {
        let mut draw = DeviceDraw::default();
        let mut rng = Rng::new(seed);
        let first = draw.draw(&mut rng, NetworkKind::Supermarket);
        let later: Vec<DeviceKind> = (0..200)
            .map(|_| draw.draw(&mut rng, NetworkKind::Supermarket))
            .collect();

        assert!(
            later
                .iter()
                .all(|kind| matches!(kind, DeviceKind::Computer | DeviceKind::Camera)),
            "{first:?} then {later:?}"
        );
    }
}

/// Every device kind is drawn, with a computer twice as often as each camera or appliance (weights 70 and 35).
#[test]
fn should_weight_the_candidates() {
    let mut rng = Rng::new(3);
    let mut counts = [0_u32; 4];

    for _ in 0..20_000 {
        let mut draw = DeviceDraw::default();
        let index = match draw.draw(&mut rng, NetworkKind::Bank) {
            DeviceKind::Computer => 0,
            DeviceKind::Camera => 1,
            DeviceKind::Fridge => 2,
            DeviceKind::Microwave => 3,
            other => panic!("{other:?}"),
        };

        counts[index] += 1;
    }

    assert!((7_000..=9_000).contains(&counts[0]), "{counts:?}");
    assert!(
        counts[1..]
            .iter()
            .all(|count| (3_300..=4_700).contains(count)),
        "{counts:?}"
    );
}

/// A network has six cameras at most (`GlobalLan.camIndex`).
#[test]
fn should_stop_drawing_cameras_after_six() {
    let mut draw = DeviceDraw::default();
    let mut rng = Rng::new(11);

    for _ in 0..6 {
        draw.add_camera();
    }

    assert_eq!(draw.cameras(), 6);

    for _ in 0..200 {
        assert_eq!(draw.draw(&mut rng, NetworkKind::Bank), DeviceKind::Computer);
    }
}

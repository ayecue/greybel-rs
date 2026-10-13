use greyhack_behaviour::{Rng, fnv1a_64};

#[test]
fn should_keep_the_numbers_of_a_seed() {
    let mut rng = Rng::new(1_477_776_061_723_855_037);
    let numbers = [rng.next_u64(), rng.next_u64(), rng.next_u64()];

    assert_eq!(
        numbers,
        [
            1_985_237_415_132_408_290,
            2_979_275_885_539_914_483,
            13_511_426_838_097_143_398
        ]
    );
}

#[test]
fn should_resume_from_its_state() {
    let mut rng = Rng::new(1_477_776_061_723_855_037);
    let _ = [rng.next_u64(), rng.next_u64(), rng.next_u64()];
    let resumed = Rng::new(rng.state());

    assert_eq!(resumed, rng);
}

#[test]
fn should_fork_from_its_next_number() {
    let mut from_seed = Rng::new(7);
    let expected = Rng::new(from_seed.next_u64() ^ 0x8594_4171_f739_67e8);
    let forked = Rng::new(7).fork("foobar");

    assert_eq!(forked, expected);
}

#[test]
fn should_draw_the_same_numbers_from_the_same_seed() {
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    let numbers = (a.next_u64(), b.next_u64());

    assert_eq!(numbers.0, numbers.1);
}

#[test]
fn should_fork_the_same_from_the_same_seed() {
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    let _ = (a.next_u64(), b.next_u64());
    let forks = (a.fork("x"), b.fork("x"));

    assert_eq!(forks.0, forks.1);
}

#[test]
fn should_fork_differently_by_name() {
    let mut a = Rng::new(7);
    let _ = a.next_u64();
    let _ = a.fork("x");
    let forks = (a.clone().fork("x"), a.fork("y"));

    assert_ne!(forks.0, forks.1);
}

#[test]
fn should_draw_ranges_and_public_ips_within_bounds() {
    let mut rng = Rng::new(1);
    let draws: Vec<_> = (0..1000)
        .map(|_| (rng.range(3, 5), rng.public_ip()))
        .collect();

    for (number, ip) in draws {
        assert!((3..=5).contains(&number));
        assert!(
            greyhack_behaviour::is_valid_ip(&ip) && !greyhack_behaviour::is_lan_ip(&ip),
            "{ip}"
        );
    }
}

#[test]
fn should_hash_text_with_fnv1a() {
    assert_eq!(fnv1a_64(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a_64("a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn should_separate_the_parts_of_a_stable_key() {
    assert_ne!(Rng::stable(1, &["ab", "c"]), Rng::stable(1, &["a", "bc"]));
    assert_eq!(Rng::stable(1, &["a", "b"]), Rng::stable(1, &["a", "b"]));
}

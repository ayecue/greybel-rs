use super::FixedPoint;

#[test]
fn should_divide_with_rounding_like_ft_divfix() {
    let (pixels, units_per_em) = (42 * 16 * 64, 1000);

    let scale = FixedPoint::ratio(pixels, units_per_em);

    assert_eq!(scale.raw(), ((pixels << 16) + 500) / 1000);
}

#[test]
fn should_round_halves_away_from_zero_like_ft_mulfix() {
    let half = FixedPoint::ratio(1, 2);

    let scaled = [half.scale(1), half.scale(-1), half.scale(3), half.scale(-3)];

    assert_eq!(scaled, [1, -1, 2, -2]);
}

#[test]
fn should_read_f2dot14_as_sixteen_dot_sixteen() {
    let one = 0x4000;

    let fixed = FixedPoint::from_f2dot14(one);

    assert_eq!(fixed.scale(1234), 1234);
}

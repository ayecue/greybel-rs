use super::{parse, round};

#[test]
fn should_round_negative_places_to_powers_of_ten() {
    let numbers = [12345.0, 12500.0, 13500.0];
    let [first, second, third] = numbers.map(|number| round(number, -3));

    assert_eq!(first, 12000.0);
    assert_eq!(second, 12000.0);
    assert_eq!(third, 14000.0);
}

#[test]
fn should_stop_places_at_fifteen() {
    let number = 0.123_456_789_012_345_67;
    let at_fifteen = round(number, 15);
    let rounded = round(number, 20);

    assert_eq!(rounded, at_fifteen);
}

#[test]
fn should_read_text_that_is_no_number_as_zero() {
    let texts = ["1,000.5", "infinity", " NaN "];
    let [grouped, infinity, nan] = texts.map(parse);

    assert_eq!(grouped, 1000.5);
    assert_eq!(infinity, 0.0);
    assert!(nan.is_nan());
}

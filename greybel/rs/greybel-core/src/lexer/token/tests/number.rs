use super::*;

#[path = "support/number.rs"]
mod support;

#[test]
fn should_accept_exactly_the_numbers_str_parse_reads() {
    let texts = support::number_texts(7);

    let mismatches: Vec<_> = texts
        .iter()
        .filter_map(|text| {
            let (taken, shape) = support::take(text);

            (shape.is_valid() != taken.parse::<f64>().is_ok()).then(|| taken.to_owned())
        })
        .collect();

    assert!(
        texts.len() > 100_000,
        "only {} texts were checked",
        texts.len()
    );
    assert_eq!(mismatches, Vec::<String>::new());
}

#[test]
fn should_stop_a_number_at_a_sign_that_follows_no_exponent() {
    let text = "1+2";

    let (taken, shape) = support::take(text);

    assert_eq!(taken, "1");
    assert!(shape.is_valid());
}

#[test]
fn should_keep_a_malformed_number_as_one_token() {
    let texts = ["1.2.3", "1e5e2", "1e", "1e+", "1e5.2"];

    let shapes: Vec<_> = texts
        .iter()
        .map(|text| {
            let (taken, shape) = support::take(text);

            (taken, shape.is_valid())
        })
        .collect();

    assert_eq!(
        shapes,
        [
            ("1.2.3", false),
            ("1e5e2", false),
            ("1e", false),
            ("1e+", false),
            ("1e5.2", false),
        ]
    );
}

use super::LowerEnvelope;

#[test]
fn should_find_the_lowest_parabola_like_a_brute_force_search() {
    let values: Vec<Option<i64>> = (0..40i64)
        .map(|column| (column % 7 != 3).then_some((column * 97 + 13) % 61))
        .collect();
    let envelope = LowerEnvelope::new(&values);

    let found: Vec<Option<i64>> = (-5..90)
        .map(|position| envelope.query(position).map(|(value, _)| value))
        .collect();

    let expected: Vec<Option<i64>> = (-5..90i64)
        .map(|position| {
            values
                .iter()
                .enumerate()
                .filter_map(|(column, value)| {
                    let offset = position - (2 * column as i64 + 1);

                    value.map(|value| offset * offset + value)
                })
                .min()
        })
        .collect();

    assert_eq!(found, expected);
}

#[test]
fn should_have_nothing_to_find_without_values() {
    let envelope = LowerEnvelope::new(&[None, None]);

    let found = envelope.query(3);

    assert_eq!(found, None);
}

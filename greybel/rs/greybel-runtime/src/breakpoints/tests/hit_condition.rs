use super::HitCondition;

#[test]
fn should_parse_hit_conditions() {
    let texts = ["5", ">= 5", "==5", "=5", "> 5", "<5", "<= 5", "% 3"];

    let conditions: Vec<_> = texts.iter().map(|text| HitCondition::parse(text)).collect();

    assert_eq!(
        conditions,
        [
            Ok(HitCondition::AtLeast(5)),
            Ok(HitCondition::AtLeast(5)),
            Ok(HitCondition::Equal(5)),
            Ok(HitCondition::Equal(5)),
            Ok(HitCondition::Above(5)),
            Ok(HitCondition::Below(5)),
            Ok(HitCondition::AtMost(5)),
            Ok(HitCondition::Every(3)),
        ]
    );
}

#[test]
fn should_reject_hit_conditions_that_are_no_count() {
    let word = HitCondition::parse("often");
    let zero = HitCondition::parse("% 0");

    assert!(word.is_err());
    assert!(zero.is_err());
}

#[test]
fn should_hold_by_the_hit_count() {
    let every_third = HitCondition::Every(3);
    let fifth = HitCondition::Equal(5);

    let thirds: Vec<u32> = (1..=9).filter(|hits| every_third.is_met(*hits)).collect();
    let fifths: Vec<u32> = (1..=9).filter(|hits| fifth.is_met(*hits)).collect();

    assert_eq!(thirds, [3, 6, 9]);
    assert_eq!(fifths, [5]);
}

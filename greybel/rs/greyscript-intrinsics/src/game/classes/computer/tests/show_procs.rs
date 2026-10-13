use super::one_decimal;

#[test]
fn should_format_like_dotnet_n1() {
    let values = [0.0, 3.04, 6.25, 1234.56, -1234567.0];
    let [zero, rounded_down, rounded_up, thousands, negative] = values.map(one_decimal);

    assert_eq!(zero, "0.0");
    assert_eq!(rounded_down, "3.0");
    assert_eq!(rounded_up, "6.3");
    assert_eq!(thousands, "1,234.6");
    assert_eq!(negative, "-1,234,567.0");
}

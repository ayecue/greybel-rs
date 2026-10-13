use super::*;

#[test]
fn should_match_scalar_binary_for_numbers() {
    use BinaryOp::*;

    let values = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.5,
        7.0,
        -7.0,
        3.0,
        1e300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    let operators = [
        Add,
        Subtract,
        Multiply,
        Divide,
        Modulo,
        Power,
        Equal,
        NotEqual,
        Less,
        LessEqual,
        Greater,
        GreaterEqual,
    ];
    let mut outcomes = Vec::new();

    for operator in operators {
        for &x in &values {
            for &y in &values {
                let fast =
                    Value::apply_numbers(operator, Number::new(x), Number::new(y)).to_number();
                let general = Value::scalar_binary(operator, &Value::from(x), &Value::from(y))
                    .expect("numbers need no heap")
                    .to_number();

                outcomes.push((operator, x, y, fast, general));
                if let Some(holds) =
                    Value::compare_numbers(operator, Number::new(x), Number::new(y))
                {
                    let compared = if holds { 1.0 } else { 0.0 };

                    outcomes.push((operator, x, y, compared, general));
                }
            }
        }
    }

    for (operator, x, y, fast, general) in outcomes {
        assert!(
            fast.to_bits() == general.to_bits() || (fast.is_nan() && general.is_nan()),
            "{operator:?} {x} {y}: {fast} vs {general}"
        );
    }
}

#[test]
fn should_concatenate_numbers_as_they_display() {
    let numbers = [
        0.0,
        -0.0,
        7.0,
        -12.0,
        2.5,
        1e20,
        1e-9,
        123_456_789_012_345.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    let prefix = Value::from("é");

    for number in numbers {
        let value = Value::from(number);
        let after = Value::concat(&prefix, &value, |_, _| {});
        let before = Value::concat(&value, &Value::Null, |_, _| {});
        let (Value::String(after), Value::String(before)) = (after, before) else {
            panic!("concatenation makes a string");
        };
        let shown = Number::new(number).to_string();

        assert_eq!(&*after, format!("é{shown}"));
        assert!(!after.is_ascii());
        assert_eq!(&*before, shown);
        assert_eq!(before.is_ascii(), shown.is_ascii());
    }
}

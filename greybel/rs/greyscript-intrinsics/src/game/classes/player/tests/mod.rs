use super::*;

#[test]
fn should_give_keys_their_unity_names() {
    let keys = [
        Key::Up,
        Key::F(5),
        Key::Shift(Side::Left),
        Key::Control(Side::Right),
        Key::Enter,
    ];
    let [up, f5, left_shift, right_control, enter] = keys.map(input);

    assert_eq!(up.as_deref(), Some("UpArrow"));
    assert_eq!(f5.as_deref(), Some("F5"));
    assert_eq!(left_shift.as_deref(), Some("LeftShift"));
    assert_eq!(right_control.as_deref(), Some("RightControl"));
    assert_eq!(enter.as_deref(), Some(""));
}

#[test]
fn should_give_characters_themselves_unless_the_terminal_ignores_them() {
    let keys = ['A', ' ', '?', '$', '~', '`'].map(Key::Char);
    let [letter, space, question, dollar, tilde, backtick] = keys.map(input);

    assert_eq!(letter.as_deref(), Some("A"));
    assert_eq!(space.as_deref(), Some(" "));
    assert_eq!(question.as_deref(), Some("?"));
    assert_eq!(dollar.as_deref(), Some("$"));
    assert_eq!(tilde, None);
    assert_eq!(backtick, None);
}

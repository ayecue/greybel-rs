use super::*;

#[test]
fn should_classify_space_separators_like_dotnet() {
    let codes = [0x20, 0xA0, 0x2003, 0x3000, 0x2028, 0x09, 0x200B, 0x1F600];

    let separators = codes.map(LineBreak::is_separator);

    assert_eq!(
        separators,
        [true, true, true, true, true, false, false, false]
    );
}

#[test]
fn should_read_the_leading_and_following_characters_of_tmp_settings() {
    let codes = ['(', '\u{300c}', ')', '\u{3002}', 'a'].map(u32::from);

    let classes = codes.map(|code| (LineBreak::is_leading(code), LineBreak::is_following(code)));

    assert_eq!(
        classes,
        [
            (true, false),
            (true, false),
            (false, true),
            (false, true),
            (false, false)
        ]
    );
}

#[test]
fn should_break_between_east_asian_characters_only() {
    let codes = [
        '\u{3053}', '\u{4f60}', '\u{ac00}', '\u{ac01}', 'a', '\u{e01}',
    ]
    .map(u32::from);

    let east_asian = codes.map(LineBreak::is_east_asian);

    assert_eq!(east_asian, [true, true, false, true, false, false]);
}

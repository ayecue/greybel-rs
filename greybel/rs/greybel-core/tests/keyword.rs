use greybel_core::Keyword;

use crate::support::keyword::keywords;

#[test]
fn should_join_two_word_keywords_across_spaces_and_tabs() {
    let sources = ["end if", "end \t  while", "else   if"];

    let [end_if, end_while, else_if] = sources.map(keywords);

    assert_eq!(end_if, Ok(vec![Keyword::EndIf]));
    assert_eq!(end_while, Ok(vec![Keyword::EndWhile]));
    assert_eq!(else_if, Ok(vec![Keyword::ElseIf]));
}

#[test]
fn should_require_a_whole_second_word() {
    let sources = ["else iffy", "else x", "else"];

    let [else_iffy, else_x, alone] = sources.map(keywords);

    assert_eq!(else_iffy, Ok(vec![Keyword::Else]));
    assert_eq!(else_x, Ok(vec![Keyword::Else]));
    assert_eq!(alone, Ok(vec![Keyword::Else]));
}

#[test]
fn should_reject_end_without_a_second_word() {
    let sources = ["end", "end foo", "end iff"];
    let expected = "'end' without related keyword ('if', 'function', etc.)";

    let [alone, end_foo, end_iff] = sources.map(keywords);

    assert_eq!(alone, Err(expected.to_owned()));
    assert_eq!(end_foo, Err(expected.to_owned()));
    assert_eq!(end_iff, Err(expected.to_owned()));
}

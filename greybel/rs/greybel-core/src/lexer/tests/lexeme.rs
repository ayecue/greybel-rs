use super::*;
use crate::lexer::{KEYWORDS, PUNCTUATORS};

#[test]
fn should_prefer_the_longest_matching_prefix() {
    let index = LexemeTable::new(&[PUNCTUATORS]);
    let texts: [&[u8]; 4] = [b"<= 1", b"< 1", b"#", b""];

    let [less_equal, less, hash, empty] = texts.map(|text| index.longest_prefix(text));

    assert_eq!(less_equal, Some(Punct::LessEqual));
    assert_eq!(less, Some(Punct::Less));
    assert_eq!(hash, None);
    assert_eq!(empty, None);
}

#[test]
fn should_get_whole_words_only() {
    let index = LexemeTable::new(&[KEYWORDS]);
    let texts = ["if", "end if", "i", "iff"];

    let [if_, end_if, i, iff] = texts.map(|text| index.get(text));

    assert_eq!(if_, Some(Keyword::If));
    assert_eq!(end_if, Some(Keyword::EndIf));
    assert_eq!(i, None);
    assert_eq!(iff, None);
}

#[test]
fn should_start_a_compound_keyword_with_end() {
    let compounds = CompoundKeywordTable::new(&[KEYWORDS]);

    let end = compounds.start(Keyword::End).unwrap();

    assert!(!end.alone);
    assert_eq!(end.keyword_for("if"), Some(Keyword::EndIf));
    assert_eq!(end.keyword_for("iff"), None);
}

#[test]
fn should_start_a_compound_keyword_with_else_that_may_stand_alone() {
    let compounds = CompoundKeywordTable::new(&[KEYWORDS]);

    let other = compounds.start(Keyword::Else).unwrap();

    assert!(other.alone);
    assert_eq!(other.keyword_for("if"), Some(Keyword::ElseIf));
}

#[test]
fn should_start_no_compound_keyword_with_a_single_word_text() {
    let compounds = CompoundKeywordTable::new(&[KEYWORDS]);

    let start = compounds.start(Keyword::If);

    assert!(start.is_none());
}

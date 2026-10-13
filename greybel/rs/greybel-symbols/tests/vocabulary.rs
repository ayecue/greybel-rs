use std::collections::HashSet;

use greybel_symbols::annotations::Tag;
use greybel_symbols::{Keyword, Precedence, Punct};

use crate::support::parsed_from_texts;

#[test]
fn should_parse_every_keyword_from_its_text() {
    let keywords = Keyword::ALL;

    let parsed = parsed_from_texts(keywords, Keyword::as_str, Keyword::parse);

    let expected: Vec<_> = keywords.iter().copied().map(Some).collect();

    assert_eq!(parsed, expected);
}

#[test]
fn should_give_every_keyword_its_own_text() {
    let keywords = Keyword::ALL;

    let texts: HashSet<_> = keywords.iter().map(|keyword| keyword.as_str()).collect();

    assert_eq!(texts.len(), Keyword::COUNT);
}

#[test]
fn should_parse_every_punctuator_from_its_text() {
    let puncts = Punct::ALL;

    let parsed = parsed_from_texts(puncts, Punct::as_str, Punct::parse);

    let expected: Vec<_> = puncts.iter().copied().map(Some).collect();

    assert_eq!(parsed, expected);
}

#[test]
fn should_give_every_punctuator_its_own_text() {
    let puncts = Punct::ALL;

    let texts: HashSet<_> = puncts.iter().map(|punct| punct.as_str()).collect();

    assert_eq!(texts.len(), Punct::COUNT);
}

#[test]
fn should_parse_every_precedence_level_from_its_name() {
    let levels = Precedence::ALL;

    let parsed = parsed_from_texts(levels, Precedence::as_str, Precedence::parse);

    let expected: Vec<_> = levels.iter().copied().map(Some).collect();

    assert_eq!(parsed, expected);
}

#[test]
fn should_give_every_precedence_level_its_own_name() {
    let levels = Precedence::ALL;

    let names: HashSet<_> = levels.iter().map(|level| level.as_str()).collect();

    assert_eq!(names.len(), Precedence::COUNT);
}

#[test]
fn should_parse_every_annotation_tag_from_its_text() {
    let tags = Tag::ALL;

    let parsed = parsed_from_texts(tags, Tag::as_str, Tag::parse);

    let expected: Vec<_> = tags.iter().copied().map(Some).collect();

    assert_eq!(parsed, expected);
}

#[test]
fn should_give_every_annotation_tag_its_own_text() {
    let tags = Tag::ALL;

    let texts: HashSet<_> = tags.iter().map(|tag| tag.as_str()).collect();

    assert_eq!(texts.len(), Tag::COUNT);
}

#[test]
fn should_index_symbols_in_their_listed_order() {
    let keywords = Keyword::ALL;
    let puncts = Punct::ALL;

    let keyword_indexes: Vec<_> = keywords.iter().map(|keyword| keyword.index()).collect();
    let punct_indexes: Vec<_> = puncts.iter().map(|punct| punct.index()).collect();

    assert_eq!(keyword_indexes, (0..Keyword::COUNT).collect::<Vec<_>>());
    assert_eq!(punct_indexes, (0..Punct::COUNT).collect::<Vec<_>>());
}

#[test]
fn should_parse_no_symbol_from_an_unknown_text() {
    let text = "unless";

    let parsed = (Keyword::parse(text), Punct::parse(text));

    assert_eq!(parsed, (None, None));
}

#[test]
fn should_take_one_byte() {
    let sizes = [
        size_of::<Keyword>(),
        size_of::<Punct>(),
        size_of::<Precedence>(),
        size_of::<Tag>(),
    ];

    let largest = sizes.into_iter().max();

    assert_eq!(largest, Some(1));
}

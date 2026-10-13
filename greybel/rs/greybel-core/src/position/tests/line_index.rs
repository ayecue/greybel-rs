#[path = "support/line_index.rs"]
mod support;

use support::{ROUND_TRIP_SOURCE, UTF16, char_by_char, padded_sources, round_trip_columns};

use super::*;

#[test]
fn should_number_lines_after_each_lexer_line_break() {
    let source = "a\nb\r\nc\rd\n\re";
    let index = LineIndex::new(source);
    let offsets: Vec<u32> = "abcde"
        .chars()
        .map(|c| source.find(c).unwrap() as u32)
        .collect();

    let lines: Vec<_> = offsets.iter().map(|&at| index.line(at)).collect();

    assert_eq!(lines, [1, 2, 3, 4, 5]);
}

#[test]
fn should_count_the_lines() {
    let source = "a\nb\r\nc\rd\n\re";

    let index = LineIndex::new(source);

    assert_eq!(index.line_count(), 5);
}

#[test]
fn should_find_where_a_line_starts() {
    let index = LineIndex::new("a\nb\r\nc\rd\n\re");

    let [third, sixth] = [3, 6].map(|line| index.line_start(line));

    assert_eq!(third, Some(5));
    assert_eq!(sixth, None);
}

#[test]
fn should_count_columns_in_the_encoding() {
    let source = "é😀x";
    let index = LineIndex::new(source);
    let x = source.find('x').unwrap() as u32;
    let encodings = [
        PositionEncoding::Utf8,
        PositionEncoding::Utf16,
        PositionEncoding::Utf32,
    ];

    let [utf8, utf16, utf32] = encodings.map(|encoding| {
        index
            .position(x, ColumnSettings::new(encoding, 1))
            .character
    });

    assert_eq!(utf8, 7);
    assert_eq!(utf16, 4);
    assert_eq!(utf32, 3);
}

#[test]
fn should_advance_tabs_by_the_tab_width() {
    let index = LineIndex::new("x\n\t\tab");
    let a = 4;
    let columns = [UTF16, ColumnSettings::new(PositionEncoding::Utf16, 4)];

    let [width_1, width_4] = columns.map(|columns| index.position(a, columns));

    assert_eq!(width_1, Position::new(2, 3));
    assert_eq!(width_4, Position::new(2, 9));
}

#[test]
fn should_invert_the_position_of_every_character() {
    let source = ROUND_TRIP_SOURCE;
    let index = LineIndex::new(source);
    let cases: Vec<_> = round_trip_columns()
        .into_iter()
        .flat_map(move |columns| {
            source
                .char_indices()
                .filter(move |&(at, _)| {
                    !(source[..at].ends_with('\r') && source[at..].starts_with('\n'))
                })
                .map(move |(at, _)| (columns, offset(at)))
        })
        .map(|(columns, at)| (columns, at, index.position(at, columns)))
        .collect();

    let outcomes: Vec<_> = cases
        .iter()
        .map(|&(columns, at, position)| (at, position, index.offset(position, columns)))
        .collect();

    for (at, position, found) in outcomes {
        assert_eq!(found, Some(at), "{position}");
    }
}

#[test]
fn should_invert_the_end_position() {
    let source = ROUND_TRIP_SOURCE;
    let index = LineIndex::new(source);
    let end = offset(source.len());
    let positions = round_trip_columns().map(|columns| (columns, index.position(end, columns)));

    let outcomes = positions.map(|(columns, position)| index.offset(position, columns));

    for found in outcomes {
        assert_eq!(found, Some(end));
    }
}

#[test]
fn should_clamp_offsets_to_the_line() {
    let index = LineIndex::new("😀x\nab");
    let positions = [
        Position::new(1, 2),
        Position::new(1, 3),
        Position::new(1, 0),
        Position::new(1, 40),
        Position::new(2, 40),
        Position::new(3, 1),
        Position::new(0, 1),
    ];

    let [
        inside_emoji,
        after_emoji,
        column_0,
        past_line_1,
        past_line_2,
        line_3,
        line_0,
    ] = positions.map(|position| index.offset(position, UTF16));

    assert_eq!(inside_emoji, Some(0));
    assert_eq!(after_emoji, Some(4));
    assert_eq!(column_0, Some(0));
    assert_eq!(past_line_1, Some(5));
    assert_eq!(past_line_2, Some(8));
    assert_eq!(line_3, None);
    assert_eq!(line_0, None);
}

#[test]
fn should_give_an_open_span_no_end_position() {
    let index = LineIndex::new("abc");
    let span = Span::empty(1).open();

    let range = index.range(span, UTF16);

    assert_eq!(range, Range::new(Position::new(1, 2), Position::default()));
}

#[test]
fn should_index_long_plain_runs_like_a_char_by_char_scan() {
    let sources = padded_sources();

    let indexes: Vec<LineIndex> = sources
        .iter()
        .map(|source| LineIndex::new(source))
        .collect();

    for (source, index) in sources.iter().zip(&indexes) {
        assert_eq!(*index, char_by_char(source), "{source:?}");
    }
}

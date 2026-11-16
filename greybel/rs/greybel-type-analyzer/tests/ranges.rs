//! Tests folding ranges and selection ranges.

use greybel_core::Position;
use greybel_type_analyzer::ColorInformation;
use greybel_type_analyzer::FoldKind::{Block, Comment, Imports};

use crate::support::ranges::{colors, folds, selections};

#[test]
fn should_fold_blocks_up_to_their_last_statement() {
    let source = "f = function(a)\n  if a then\n    print 1\n  else\n    print 2\n  end if\n  for i in [1]\n    print i\n  end for\nend function\nwhile true\n  break\nend while";

    assert_eq!(
        folds(source),
        [
            (1, 9, Block),
            (2, 3, Block),
            (4, 5, Block),
            (7, 8, Block),
            (11, 12, Block)
        ]
    );
}

#[test]
fn should_not_fold_single_line_blocks() {
    let source = "if true then print 1\nf = function(a)\nend function\nx = [1, 2]";

    assert_eq!(folds(source), []);
}

#[test]
fn should_fold_lists_and_maps_over_several_lines() {
    let source = "m = {\n  \"a\": [\n    1,\n  ],\n}";

    assert_eq!(folds(source), [(1, 4, Block), (2, 3, Block)]);
}

#[test]
fn should_fold_comment_lines_and_import_lines() {
    let source = "// one\n// two\nx = 1 // trailing\n#include \"a.src\"\n\n#include \"b.src\"\nprint 1\n#include \"c.src\"";

    assert_eq!(folds(source), [(1, 2, Comment), (4, 6, Imports)]);
}

#[test]
fn should_select_the_nodes_around_a_position() {
    let source = "x = foo(1 + 2)";

    assert_eq!(
        selections(source, Position::new(1, 9)),
        ["1", "1 + 2", "foo(1 + 2)", "x = foo(1 + 2)"]
    );
}

#[test]
fn should_find_the_color_values_of_tags_in_strings() {
    let source = "print \"<color=#FF8000>a</color> <mark=#ff000080>b</mark> <#0f0>c <color=\"\"red\"\">d <color=teal>e\"\nx = \"<color=blue>\" + \"#fff\"";

    assert_eq!(
        colors(source),
        [
            ((1, 15), (255, 128, 0, 255)),
            ((1, 39), (255, 0, 0, 128)),
            ((1, 59), (0, 255, 0, 255)),
            ((1, 75), (255, 0, 0, 255)),
            ((2, 13), (0, 0, 255, 255)),
        ]
    );
}

#[test]
fn should_write_colors_as_hex() {
    assert_eq!(ColorInformation::hex(255, 128, 0, 255), "#FF8000");
    assert_eq!(ColorInformation::hex(255, 0, 0, 128), "#FF000080");
}

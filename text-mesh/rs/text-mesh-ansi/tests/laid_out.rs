//! Rendering through text-mesh-layout: word wrapping, margins, alignment and `<mspace>` carry over to the terminal.

use crate::support::render_laid_out;

#[test]
fn should_wrap_words_at_the_width() {
    let text = "one two three four";
    let shown = render_laid_out(text, 10);

    assert_eq!(shown, "one two\nthree four");
}

#[test]
fn should_break_a_word_longer_than_the_width() {
    let text = "abcdefghijkl";
    let shown = render_laid_out(text, 8);

    assert_eq!(shown, "abcdefgh\nijkl");
}

#[test]
fn should_indent_a_line_by_its_margin() {
    let text = "<margin=20%>ab</margin>";
    let shown = render_laid_out(text, 20);

    assert_eq!(shown, "    ab");
}

#[test]
fn should_align_a_line_to_the_right() {
    let text = "<align=right>ab";
    let shown = render_laid_out(text, 10);

    assert_eq!(shown, "        ab");
}

#[test]
fn should_space_monospaced_characters_apart() {
    let text = "<mspace=3em>ab";
    let shown = render_laid_out(text, 20);

    assert_eq!(shown, "  a    b");
}

#[test]
fn should_draw_a_caret_over_the_cell_it_does_not_advance_past() {
    let text = "a<sprite=0>bc";
    let shown = render_laid_out(text, 20);

    assert_eq!(shown, "a█c");
}

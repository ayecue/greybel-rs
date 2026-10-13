//! Layout tags that move the pen or transform glyphs.

use text_mesh_core::{Length, Run};

use crate::support::{BASE_SIZE, parse, pen_moves, plain, style_of};

#[test]
fn should_move_the_pen_to_a_position_in_pixels() {
    let text = "a<pos=4.95>b";
    let shown = pen_moves(text);

    assert_eq!(shown, ["pos 4.95px"]);
}

#[test]
fn should_read_a_position_in_font_units_at_the_current_size() {
    let text = "<size=10><pos=2em>a";
    let shown = pen_moves(text);

    assert_eq!(shown, ["pos 20.00px"]);
}

#[test]
fn should_keep_a_position_in_percent_of_the_width() {
    let text = "<pos=50%>a";
    let shown = pen_moves(text);

    assert_eq!(shown, ["pos 50.00%"]);
}

#[test]
fn should_read_a_position_without_digits_as_zero_as_the_game_does() {
    let text = "<pos=x>a";
    let shown = (plain(text), pen_moves(text));

    assert_eq!(shown, ("a".to_owned(), vec!["pos 0.00px".to_owned()]));
}

#[test]
fn should_hide_a_closing_position_without_moving() {
    let text = "a</pos>b";
    let shown = (plain(text), pen_moves(text));

    assert_eq!(shown, ("ab".to_owned(), Vec::<String>::new()));
}

#[test]
fn should_add_a_space_in_pixels_or_font_units() {
    let text = "a<space=3>b<space=0.5em>c";
    let shown = pen_moves(text);

    assert_eq!(shown, ["space 3.00px", "space 10.00px"]);
}

#[test]
fn should_show_a_space_in_percent_as_text() {
    let text = "<space=5%>a";
    let shown = plain(text);

    assert_eq!(shown, text);
}

#[test]
fn should_offset_the_baseline_in_font_units_of_the_current_size() {
    let text = "<voffset=-0.35em>a</voffset>b";
    let shown = [style_of(text, "a"), style_of(text, "b")]
        .map(|style| format!("{:.2}", style.vertical_offset / BASE_SIZE));

    assert_eq!(shown, ["-0.35", "0.00"]);
}

#[test]
fn should_replace_an_offset_with_the_next_one() {
    let text = "<voffset=-0.35em><voffset=-0.7em>a";
    let shown = format!("{:.2}", style_of(text, "a").vertical_offset / BASE_SIZE);

    assert_eq!(shown, "-0.70");
}

#[test]
fn should_rotate_the_following_characters() {
    let text = "<rotate=90>a<rotate=0>b";
    let shown = [style_of(text, "a"), style_of(text, "b")].map(|style| style.rotation);

    assert_eq!(shown, [90.0, 0.0]);
}

#[test]
fn should_let_rotate_and_scale_replace_each_other() {
    let text = "<rotate=90><scale=1.5>a<rotate=45>b";
    let shown =
        [style_of(text, "a"), style_of(text, "b")].map(|style| (style.rotation, style.scale));

    assert_eq!(shown, [(0.0, 1.5), (45.0, 1.0)]);
}

#[test]
fn should_clear_rotation_and_scale_with_either_closing_tag() {
    let text = "<rotate=90>a</scale>b";
    let shown = style_of(text, "b");

    assert_eq!((shown.rotation, shown.scale), (0.0, 1.0));
}

#[test]
fn should_parse_a_prompt_drawn_with_positions_offsets_and_rotation() {
    let text =
        "<pos=4.95>——<voffset=-0.35em><pos=0><rotate=90>—<rotate=0><pos=4.95><voffset=-1.4em>—$";
    let shape: Vec<String> = parse(text)
        .runs
        .iter()
        .map(|run| match run {
            Run::Text { text, style } => format!(
                "{text}@{:.2}/{}",
                style.vertical_offset / BASE_SIZE,
                style.rotation
            ),
            Run::Position {
                x: Length::Pixels(x),
            } => format!("pos {x:.2}"),
            other => format!("{other:?}"),
        })
        .collect();

    assert_eq!(
        shape,
        [
            "pos 4.95",
            "——@0.00/0",
            "pos 0.00",
            "—@-0.35/90",
            "pos 4.95",
            "—$@-1.40/0"
        ]
    );
}

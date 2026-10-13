use crate::support::{render, render_painted, renderer};

#[test]
fn should_leave_text_in_the_component_color_plain() {
    let text = "plain";
    let shown = render(text);

    assert_eq!(shown, "plain");
}

#[test]
fn should_write_styles_as_sgr_codes() {
    let text = "<b><i><u><s>x";
    let shown = render(text);

    assert_eq!(shown, "\x1b[1;3;4;9mx\x1b[0m");
}

#[test]
fn should_write_colors_as_24_bit_codes() {
    let text = "<color=red>x</color>y";
    let shown = render(text);

    assert_eq!(shown, "\x1b[38;2;255;0;0mx\x1b[0my");
}

#[test]
fn should_blend_translucent_colors_over_the_background() {
    let text = "<color=#ff000080>a</color><mark>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "\x1b[38;2;128;0;0ma\x1b[0m\x1b[48;2;64;64;0mb\x1b[0m"
    );
}

#[test]
fn should_break_lines_draw_text_over_a_sprite_and_drop_soft_hyphens() {
    let text = "a<br>b<sprite=0>c\u{ad}d";
    let shown = render(text);

    assert_eq!(shown, "a\nbcd");
}

#[test]
fn should_not_carry_styles_over_terminal_lines() {
    let output = "<color=red>a\nb";
    let shown = renderer().render_terminal(output);

    assert_eq!(shown, "\x1b[38;2;255;0;0ma\x1b[0m\nb");
}

#[test]
fn should_reset_styles_before_each_line_break() {
    let text = "<b>a<br>b";
    let shown = render(text);

    assert_eq!(shown, "\x1b[1ma\x1b[0m\n\x1b[1mb\x1b[0m");
}

#[test]
fn should_paint_the_component_colors_when_painting_the_background() {
    let text = "a<mark=#ff0000>b";
    let shown = render_painted(text);

    assert_eq!(
        shown,
        "\x1b[38;2;255;255;255;48;2;0;0;0ma\x1b[0m\x1b[38;2;255;255;255;48;2;255;0;0mb\x1b[0m"
    );
}

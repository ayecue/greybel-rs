use super::*;

#[path = "support/tags.rs"]
mod support;

#[test]
fn should_return_the_previous_item_when_removing_from_a_processing_stack() {
    let mut stack = ValueStack::new(Alignment::Left);

    stack.add(Alignment::Center);
    stack.add(Alignment::Right);

    let removed = [stack.remove(), stack.remove(), stack.remove()];

    assert_eq!(
        removed,
        [Alignment::Center, Alignment::Left, Alignment::Left]
    );
}

#[test]
fn should_resolve_a_margin_percentage_against_the_width_left_by_a_width_tag() {
    let fonts = support::font_set();
    let context = support::tag_context(&fonts, 1000.0, None);
    let mut tags = TagState::new(Alignment::Left);

    tags.apply(&LayoutChange::Width(Length::Pixels(600.0)), &context);

    tags.apply(
        &LayoutChange::Margin {
            left: Some(Length::Percentage(10.0)),
            right: Some(Length::Pixels(-5.0)),
        },
        &context,
    );

    assert_eq!((tags.margin_left, tags.margin_right), (40.0, 0.0));
}

#[test]
fn should_move_the_pen_to_an_indent() {
    let fonts = support::font_set();
    let context = support::tag_context(&fonts, 980.0, None);
    let mut tags = TagState::new(Alignment::Left);

    tags.x_advance = 120.0;

    tags.apply(&LayoutChange::Indent(Length::Percentage(15.0)), &context);

    assert_eq!((tags.x_advance, tags.indent), (147.0, 147.0));
}

#[test]
fn should_scale_a_line_height_percentage_by_the_primary_line_height() {
    let fonts = support::font_set();
    let context = support::tag_context(&fonts, 980.0, Some(1));
    let mut tags = TagState::new(Alignment::Left);

    tags.apply(
        &LayoutChange::LineHeight(LineHeight::Percentage {
            value: 50.0,
            size: 16.0,
            font: Some(1),
        }),
        &context,
    );

    assert_eq!(tags.line_height, Some(50.0 * 0.5 * (16.0 / 80.0)));
}

#[test]
fn should_limit_the_text_area_to_a_width_tag() {
    let mut tags = TagState::new(Alignment::Left);

    tags.margin_left = 100.0;
    tags.width = Some(500.0);

    let areas = [
        tags.width_of_text_area(980.0),
        tags.width_of_text_area(550.0),
    ];

    assert_eq!(areas, [500.0, 550.0001 - 100.0]);
}

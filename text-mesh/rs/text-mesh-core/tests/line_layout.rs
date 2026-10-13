//! Line layout tags, recorded for the layout, which resolves percentages against the widths it knows.

use text_mesh_core::{Alignment, LayoutChange, Length, LineHeight};

use crate::support::{layout_changes, plain};

#[test]
fn should_record_a_margin_on_both_sides() {
    let text = "<margin=10%>a";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [LayoutChange::Margin {
            left: Some(Length::Percentage(10.0)),
            right: Some(Length::Percentage(10.0)),
        }]
    );
}

#[test]
fn should_read_margin_attributes_with_their_own_units() {
    let text = "<margin left=2em right=5%>a";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [LayoutChange::Margin {
            left: Some(Length::Pixels(40.0)),
            right: Some(Length::Percentage(5.0)),
        }]
    );
}

#[test]
fn should_record_a_left_margin_only() {
    let text = "<margin-left=5>a";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [LayoutChange::Margin {
            left: Some(Length::Pixels(5.0)),
            right: None,
        }]
    );
}

#[test]
fn should_end_both_margins() {
    let text = "<margin=5>a</margin>b";
    let changes = layout_changes(text);

    assert_eq!(changes[1], LayoutChange::MarginEnd);
}

#[test]
fn should_show_a_closing_margin_left_as_text() {
    let text = "a</margin-left>";
    let shown = plain(text);

    assert_eq!(shown, "a</margin-left>");
}

#[test]
fn should_record_an_alignment_and_its_end() {
    let text = "<align=center>a</align>";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [
            LayoutChange::Align(Alignment::Center),
            LayoutChange::AlignEnd
        ]
    );
}

#[test]
fn should_show_an_unknown_alignment_as_text() {
    let text = "<align=middle>a";
    let shown = plain(text);

    assert_eq!(shown, "<align=middle>a");
}

#[test]
fn should_show_a_width_in_font_units_as_text() {
    let text = "<width=2em>a";
    let shown = plain(text);

    assert_eq!(shown, "<width=2em>a");
}

#[test]
fn should_keep_the_size_and_font_of_a_line_height_percentage() {
    let text = "<size=10><font=\"LiberationSans SDF\"><line-height=50%>a";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [LayoutChange::LineHeight(LineHeight::Percentage {
            value: 50.0,
            size: 10.0,
            font: Some(0),
        })]
    );
}

#[test]
fn should_read_an_indent_in_font_units_at_the_current_size() {
    let text = "<size=10><indent=2em>a";
    let changes = layout_changes(text);

    assert_eq!(changes, [LayoutChange::Indent(Length::Pixels(20.0))]);
}

#[test]
fn should_record_where_breaking_stops_and_starts_again() {
    let text = "<nobr>a b</nobr> c";
    let changes = layout_changes(text);

    assert_eq!(
        changes,
        [LayoutChange::NoBreak(true), LayoutChange::NoBreak(false)]
    );
}

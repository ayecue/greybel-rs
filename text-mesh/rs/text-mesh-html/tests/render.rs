use crate::support::{render, renderer};

#[test]
fn should_escape_text_shown_as_is() {
    let text = "<foo>&\"'";
    let shown = render(text);

    assert_eq!(shown, "&lt;foo&gt;&amp;&quot;&#39;");
}

#[test]
fn should_wrap_styled_text_in_spans() {
    let text = "<b><color=#ff000080>x";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"color:rgba(255,0,0,0.502);font-weight:700\">x</span>"
    );
}

#[test]
fn should_write_decorations_and_highlights() {
    let text = "<u color=#00ff00><s><mark>x";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"text-decoration-line:underline line-through;text-decoration-color:rgb(0,255,0);background-color:rgba(255,255,0,0.251)\">x</span>"
    );
}

#[test]
fn should_size_relative_to_the_component() {
    let text = "<size=30>a</size><smallcaps>b";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"font-size:1.5em\">a</span><span style=\"font-size:0.8em\">B</span>"
    );
}

#[test]
fn should_write_breaks_and_sprites() {
    let text = "a<br><sprite=1>";
    let shown = render(text);

    assert_eq!(
        shown,
        "a<br><span class=\"sprite\" data-index=\"1\" style=\"color:rgb(255,255,255)\"></span>"
    );
}

#[test]
fn should_put_each_terminal_line_in_its_own_element() {
    let output = "<b>a\nb";
    let shown = renderer().render_terminal(output);

    assert_eq!(
        shown,
        "<div class=\"line\"><span style=\"font-weight:700\">a</span></div><div class=\"line\">b</div>"
    );
}

#[test]
fn should_name_the_font_of_a_font_tag_as_a_quoted_family() {
    let text = "<font=\"Liberation's Sans\">a";
    let shown = render(text);

    assert_eq!(
        shown,
        "<span style=\"font-family:'Liberation\\'s Sans'\">a</span>"
    );
}

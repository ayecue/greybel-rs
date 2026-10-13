use super::format_columns_text;

#[test]
fn should_format_columns_like_grey_hack() {
    let texts = ["NAME SIZE\\nkernel.img 12\na b", "a  b"];
    let [table, double_space] = texts.map(format_columns_text);

    assert_eq!(
        table,
        "NAME        SIZE  \nkernel.img  12    \na           b"
    );
    assert_eq!(double_space, "a    b");
}

#[test]
fn should_leave_rich_text_tags_out_of_a_columns_width() {
    let table = format_columns_text("<b>A</b> B\nC D");

    assert_eq!(table, "<b>A</b>  B  \nC  D");
}

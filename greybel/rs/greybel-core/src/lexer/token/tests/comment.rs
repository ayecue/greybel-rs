#[path = "support/comment.rs"]
mod support;

use support::block_comment;

#[test]
fn should_end_a_block_comment_only_after_its_opener() {
    let sources = ["/*/", "/*/ x", "/**/", "/*/*/", "/* a */"];

    let [unclosed, unclosed_with_text, empty, slash, text] = sources.map(block_comment);

    assert!(unclosed.is_err());
    assert!(unclosed_with_text.is_err());
    assert_eq!(empty.as_deref(), Ok(""));
    assert_eq!(slash.as_deref(), Ok("/"));
    assert_eq!(text.as_deref(), Ok(" a "));
}

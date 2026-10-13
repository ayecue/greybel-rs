use super::ThinStr;

#[test]
fn should_hold_the_text_and_header() {
    let text = ThinStr::new(true, "abc");

    assert_eq!(&*text, "abc");
    assert!(text.header());
    assert_eq!(text.len(), 3);
}

#[test]
fn should_hold_empty_and_unicode_texts() {
    let empty = ThinStr::new((), "");

    assert_eq!(&*empty, "");

    let unicode = ThinStr::new((), "aé😀");

    assert_eq!(&*unicode, "aé😀");
    assert_eq!(unicode.chars().count(), 3);
}

#[test]
fn should_join_the_parts() {
    let joined = ThinStr::concat(7_u8, &["s", "", "12", "é"]);

    assert_eq!(&*joined, "s12é");
    assert_eq!(joined.header(), 7);

    let none = ThinStr::concat(0_u8, &[]);

    assert_eq!(&*none, "");
}

#[test]
fn should_count_and_share_clones() {
    let text = ThinStr::new((), "shared");

    assert_eq!(ThinStr::strong_count(&text), 1);

    let copy = text.clone();

    assert_eq!(ThinStr::strong_count(&text), 2);
    assert!(ThinStr::ptr_eq(&text, &copy));
    drop(copy);
    assert_eq!(ThinStr::strong_count(&text), 1);

    let equal = ThinStr::new((), "shared");

    assert!(!ThinStr::ptr_eq(&text, &equal));
    assert_eq!(&*text, &*equal);
}

#[test]
fn should_keep_the_text_after_the_original_drops() {
    let copy = {
        let text = ThinStr::new(1_u64, "outlives");

        text.clone()
    };

    assert_eq!(&*copy, "outlives");
    assert_eq!(copy.header(), 1);
}

#[test]
fn should_align_a_wide_header() {
    #[derive(Clone, Copy, PartialEq, Debug)]
    #[repr(align(32))]
    struct Wide(u8);

    let text = ThinStr::new(Wide(9), "xyz");

    assert_eq!(text.header(), Wide(9));
    assert_eq!(&*text, "xyz");
}

#[test]
fn should_hold_long_texts() {
    let long = "ab".repeat(10_000);
    let text = ThinStr::concat(false, &[&long, &long]);

    assert_eq!(text.len(), 40_000);
    assert!(text.starts_with("abab") && text.ends_with("abab"));
}

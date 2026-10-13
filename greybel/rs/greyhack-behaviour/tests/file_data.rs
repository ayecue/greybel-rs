use greyhack_behaviour::{FileData, timestamp_hash};

#[test]
fn should_hash_timestamps_over_utf16_code_units() {
    assert_eq!(timestamp_hash("abc"), 440_920_331);
    assert_eq!(timestamp_hash("😀"), 3_409_036_472);
}

#[test]
fn should_recompute_text_and_nondefault_program_sizes() {
    let text = FileData::text("a😀b");
    let program = FileData::binary_from_source("a😀b");
    let mut default_program = FileData::binary_from_source("a😀b");

    default_program.default_content = true;
    default_program.size = 99;
    default_program.set_content("a😀b");

    assert_eq!(text.size, 4);
    assert_eq!(program.size, 2);
    assert_eq!(default_program.size, 99);
}

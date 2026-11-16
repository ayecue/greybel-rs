use greyhack_behaviour::{FileData, file_size_mb, stored_file_size_mb, timestamp_hash};

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

#[test]
fn should_compare_and_store_file_megabytes_at_the_csharp_precision_boundaries() {
    assert_eq!(file_size_mb(1), 9.999_999_974_752_427e-7);
    assert_eq!(stored_file_size_mb(1), 1E-6_f32);
    assert!(file_size_mb(1) <= f64::from(1E-6_f32));
    assert!(1.0_f64 / 1_000_000.0 > f64::from(1E-6_f32));
}

#[test]
fn should_truncate_the_unrounded_program_size_product_like_the_game() {
    let program = FileData::binary_from_source(&"a".repeat(2080));

    assert_eq!(program.size, 1351);
}

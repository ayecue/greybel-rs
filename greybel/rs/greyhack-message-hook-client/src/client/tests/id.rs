use super::IdGenerator;

#[test]
fn should_generate_alphanumeric_ids_that_differ() {
    let ids = IdGenerator::default();
    let (a, b) = (ids.generate(), ids.generate());

    assert_eq!(a.len(), 16);
    assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
    assert_ne!(a, b);
}

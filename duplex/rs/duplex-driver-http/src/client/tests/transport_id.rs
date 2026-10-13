use super::TransportId;

#[test]
fn should_read_back_the_id_it_writes() {
    let id = TransportId::random().expect("the system has entropy");

    let header = id.to_header();

    assert_eq!(header.len(), 32);
    assert_eq!(TransportId::parse(header.as_bytes()), Some(id));
}

#[test]
fn should_refuse_what_isnt_32_hexadecimal_digits() {
    let invalid = [&b"00"[..], &[b'0'; 33], &[b'g'; 32], b""];

    let parsed: Vec<_> = invalid
        .iter()
        .map(|text| TransportId::parse(text))
        .collect();

    assert_eq!(parsed, [None, None, None, None]);
}

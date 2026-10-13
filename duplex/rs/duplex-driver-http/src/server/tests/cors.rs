use super::{ALLOWED_HEADERS, EXPOSED_HEADERS};
use crate::client::{RECEIVED_HEADER, TRANSPORT_HEADER};

#[test]
fn should_allow_every_header_a_client_sends() {
    let allowed: Vec<&str> = ALLOWED_HEADERS.split(", ").collect();

    assert_eq!(allowed, [TRANSPORT_HEADER, RECEIVED_HEADER, "content-type"]);
}

#[test]
fn should_expose_the_transport_a_first_post_opened() {
    assert_eq!(EXPOSED_HEADERS, TRANSPORT_HEADER);
}

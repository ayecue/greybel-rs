//! Client bindings for the standalone test-support protocol.

pub mod test_support {
    duplex_core::include_protocol!("$PROTOCOLS/greyhack-test-support", major = 1, role = client);
}

pub use test_support::TestClientNotification;

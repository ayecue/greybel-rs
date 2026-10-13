//! Client-side program bindings for the I/O protocol.

duplex_core::include_protocol!(
    "$PROTOCOLS/io",
    major = 1,
    role = client,
    types = greybel_io
);

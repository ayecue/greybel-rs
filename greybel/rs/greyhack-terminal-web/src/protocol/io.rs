//! Provider-side page bindings for program output in the io protocol.

duplex_core::include_protocol!(
    "$PROTOCOLS/io",
    major = 1,
    role = provider,
    types = greybel_io
);

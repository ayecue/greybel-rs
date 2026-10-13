//! Client-side documents protocol bindings used by language-server host providers.

duplex_core::include_protocol!("$PROTOCOLS/documents", major = 1, role = client);

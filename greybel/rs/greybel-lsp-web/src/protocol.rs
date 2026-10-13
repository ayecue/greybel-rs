//! Provider-side bindings for the language-server protocol.

duplex_core::include_protocol!("$PROTOCOLS/language-server", major = 1, role = provider);

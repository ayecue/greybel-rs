use duplex_protocol::Role;
use quote::ToTokens;

use crate::input::{ProtocolInput, TypesInput};

#[test]
fn should_read_the_role_of_the_generated_peer() {
    let roles = ["client", "provider"];

    let read = roles.map(|role| {
        syn::parse_str::<ProtocolInput>(&format!(r#""io", major = 1, role = {role}"#))
            .map(|input| input.role)
    });

    assert_eq!(
        read.map(Result::ok),
        [Some(Role::Client), Some(Role::Provider)]
    );
}

#[test]
fn should_read_the_module_of_the_types() {
    let source = r#""io", role = client, types = greybel_io::types, major = 1,"#;

    let input = syn::parse_str::<ProtocolInput>(source).expect("parses");

    assert_eq!(
        input.types.map(|types| types.to_token_stream().to_string()),
        Some("greybel_io :: types".to_owned())
    );
}

#[test]
fn should_require_a_role() {
    let sources = [r#""io", major = 1"#, r#""io", major = 1, role = server"#];

    let errors = sources.map(|source| {
        syn::parse_str::<ProtocolInput>(source)
            .err()
            .map(|error| error.to_string())
    });

    let [missing, unknown] = errors;

    assert!(missing.is_some());
    assert_eq!(
        unknown,
        Some("the role is `client` or `provider`".to_owned())
    );
}

#[test]
fn should_read_where_duplex_core_is_for_types() {
    let source = r#""duplex/core", major = 1, core = crate"#;

    let input = syn::parse_str::<TypesInput>(source).expect("parses");

    assert_eq!(
        input.core.map(|core| core.to_token_stream().to_string()),
        Some("crate".to_owned())
    );
}

#[test]
fn should_read_no_core_for_types_that_name_none() {
    let source = r#""io", major = 1"#;

    let input = syn::parse_str::<TypesInput>(source).expect("parses");

    assert!(input.core.is_none());
}

#[test]
fn should_read_the_major_generated() {
    let source = r#""io", role = client, major = 2"#;

    let input = syn::parse_str::<ProtocolInput>(source).expect("parses");

    assert_eq!(input.major, 2);
}

#[test]
fn should_require_the_major_generated() {
    let source = r#""io", role = client"#;

    let error = syn::parse_str::<ProtocolInput>(source).err();

    assert_eq!(
        error.map(|error| error.to_string()),
        Some("the major generated is given, as `major = 1`".to_owned())
    );
}

//! Procedural macros re-exported by `duplex-core` to generate Rust bindings from versioned protocol definition
//! folders.

#![warn(missing_docs)]
mod generate;
mod input;

use proc_macro::TokenStream;
use syn::parse_macro_input;

use crate::generate::Generator;
use crate::input::{MigrationInput, ProtocolInput, TypesInput};

/// Generates client or provider bindings for one protocol major from a definition folder.
#[proc_macro]
pub fn include_protocol(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ProtocolInput);

    input.folder.generate(input.major, |generator| {
        generator.protocol(input.role, input.types.as_ref())
    })
}

/// Generates Rust types and binary codecs for one definition major.
#[proc_macro]
pub fn include_types(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TypesInput);

    input.folder.generate(input.major, |generator| {
        Ok(generator.with_core(input.core.as_ref()).types())
    })
}

/// Generates client migration code between consecutive protocol majors.
#[proc_macro]
pub fn include_migration(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as MigrationInput);
    let folder = &input.folder;

    folder
        .load()
        .and_then(|(versions, files)| {
            let older = folder.major(&versions, input.from)?;
            let newer = folder.major(&versions, input.from + 1)?;

            Generator::new(&files, newer).migration(older, &input.older, &input.newer)
        })
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

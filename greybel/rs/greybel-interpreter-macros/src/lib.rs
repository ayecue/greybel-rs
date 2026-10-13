mod builder;
mod options;
mod signature;

use proc_macro::TokenStream;
use syn::{ItemFn, parse_macro_input};

use crate::builder::Builder;
use crate::options::Options;

#[proc_macro_attribute]
pub fn native(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut options = Options::default();
    let parser = syn::meta::parser(|meta| options.parse(&meta));

    parse_macro_input!(attr with parser);

    let mut function = parse_macro_input!(item as ItemFn);

    Builder::from_input(options, &mut function)
        .and_then(|builder| builder.generate())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

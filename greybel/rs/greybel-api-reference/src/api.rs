use meta_utils::{BaseType, Container, DEFAULT_LANGUAGE, Signature};

/// Language used to lex examples.
pub(crate) const LANGUAGE: greybel_core::Dialect = greybel_core::Dialect::GreyScript;

/// Metadata for the GreyScript API.
pub(crate) fn container() -> &'static Container {
    greyscript_meta::container()
}

/// Returns documented types sorted by lowercase name, excluding `any`, hidden types, and types without members
/// or descriptions.
pub(crate) fn types() -> Vec<&'static Signature> {
    let mut types: Vec<&Signature> = container()
        .signatures()
        .filter(|signature| {
            signature.type_name() != BaseType::Any.as_str()
                && !signature.is_hidden()
                && (!signature.definitions().is_empty()
                    || signature
                        .descriptions(DEFAULT_LANGUAGE)
                        .and_then(|table| table.type_description())
                        .is_some())
        })
        .collect();

    types.sort_by_key(|signature| signature.type_name().to_lowercase());

    types
}

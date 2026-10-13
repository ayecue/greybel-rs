mod container;
mod definition;
mod description;
mod error;
mod signature;
mod type_meta;
mod variation;

pub use container::Container;
pub use definition::{
    Argument, ArgumentDefault, Definition, Function, LocalizedDefinition, Mutation,
};
pub use description::{Description, DescriptionTable};
pub use error::{MetaError, Result, TypeParseError};
pub use signature::{Signature, SignaturePayload};
pub use type_meta::{BaseType, MAX_TYPE_DEPTH, TypeLookup, TypeMeta};
pub use variation::{Variation, VariationRegistry};

pub const DEFAULT_LANGUAGE: &str = "en";

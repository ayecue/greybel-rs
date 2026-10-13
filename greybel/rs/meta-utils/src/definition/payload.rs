use indexmap::IndexMap;
use serde::Deserialize;

use super::{Argument, ArgumentDefault, Definition, Function, Mutation};
use crate::type_meta::{BaseType, TypeMeta};
use crate::variation::Variation;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct DefinitionPayload {
    r#type: TypeMeta,
    id: Option<String>,
    is_protected: Option<bool>,
    description: Option<String>,
    example: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    variations: Option<Vec<Variation>>,
    arguments: Option<Vec<Argument>>,
    returns: Option<Vec<TypeMeta>>,
    return_variations: Option<Vec<Variation>>,
    mutates: Option<MutatesPayload>,
}

/// Reason an API definition or argument cannot be read.
#[derive(Debug, thiserror::Error)]
pub(super) enum PayloadError {
    #[error("a function needs an `id`")]
    MissingId,
    #[error("a function needs `returns`")]
    MissingReturns,
    #[error("`{field}` is only allowed on functions, not on `{ty}`")]
    FunctionOnlyField { field: &'static str, ty: TypeMeta },
    #[error("argument `{label}` needs either `type` or `types`")]
    ArgumentTypes { label: String },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MutatesPayload {
    Flag(bool),
    Fixed(IndexMap<String, TypeMeta>),
}

impl TryFrom<DefinitionPayload> for Definition {
    type Error = PayloadError;

    fn try_from(payload: DefinitionPayload) -> Result<Self, Self::Error> {
        let function = if payload.r#type.is(BaseType::Function) {
            Some(Function {
                id: payload.id.ok_or(PayloadError::MissingId)?,
                arguments: payload.arguments.unwrap_or_default(),
                returns: payload.returns.ok_or(PayloadError::MissingReturns)?,
                return_variations: payload.return_variations.unwrap_or_default(),
                mutation: match payload.mutates {
                    None | Some(MutatesPayload::Flag(false)) => None,
                    Some(MutatesPayload::Flag(true)) => Some(Mutation::default()),
                    Some(MutatesPayload::Fixed(fixed_types)) => Some(Mutation { fixed_types }),
                },
            })
        } else {
            let function_only = [
                ("id", payload.id.is_some()),
                ("arguments", payload.arguments.is_some()),
                ("returns", payload.returns.is_some()),
                ("returnVariations", payload.return_variations.is_some()),
                ("mutates", payload.mutates.is_some()),
            ];

            if let Some((field, _)) = function_only.iter().find(|(_, present)| *present) {
                return Err(PayloadError::FunctionOnlyField {
                    field,
                    ty: payload.r#type,
                });
            }

            None
        };

        Ok(Self {
            type_meta: payload.r#type,
            is_protected: payload.is_protected.unwrap_or(false),
            description: payload.description,
            example: payload.example,
            variations: payload.variations.unwrap_or_default(),
            tags: payload.tags.unwrap_or_default(),
            function,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArgumentPayload {
    label: String,
    r#type: Option<TypeMeta>,
    types: Option<Vec<TypeMeta>>,
    opt: Option<bool>,
    default: Option<ArgumentDefault>,
}

impl TryFrom<ArgumentPayload> for Argument {
    type Error = PayloadError;

    fn try_from(payload: ArgumentPayload) -> Result<Self, Self::Error> {
        let types = match (payload.r#type, payload.types) {
            (Some(single), None) => vec![single],
            (None, Some(types)) => types,
            _ => {
                return Err(PayloadError::ArgumentTypes {
                    label: payload.label,
                });
            }
        };

        Ok(Self {
            label: payload.label,
            types,
            optional: payload.opt.unwrap_or(false),
            default: payload.default,
        })
    }
}

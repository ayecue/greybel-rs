use std::fmt;

use meta_utils::{Argument, ArgumentDefault, TypeMeta};

use super::SemanticModel;
use crate::annotation::TypeExpr;
use crate::api::Api;
use crate::types::{FunctionRef, TypeId, TypeTable};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Documentation {
    pub description: Option<String>,
    pub examples: Vec<String>,
}

pub(crate) struct FunctionSignature<'a> {
    pub parameters: Vec<SignatureParameter<'a>>,
    pub returns: SignatureType<'a>,
}

pub(crate) struct SignatureParameter<'a> {
    pub name: &'a str,
    pub ty: SignatureType<'a>,
    pub default: Option<ParameterDefault<'a>>,
    pub optional: bool,
    pub is_annotated: bool,
    pub description: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub(crate) enum ParameterDefault<'a> {
    Source(&'a str),
    Api(&'a ArgumentDefault),
}

impl fmt::Display for ParameterDefault<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(text) => f.write_str(text),
            Self::Api(ArgumentDefault::String(value)) => write!(f, "{value:?}"),
            Self::Api(ArgumentDefault::Number(value)) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SignatureType<'a> {
    Value(TypeId),
    Api(&'a [TypeMeta]),
    /// The return type `value is T` of a type guard, as written; calls of it return a number.
    Guard(&'a TypeExpr),
}

impl SemanticModel {
    pub(crate) fn signature(
        &self,
        function: FunctionRef,
        method: bool,
    ) -> Option<FunctionSignature<'_>> {
        match function {
            FunctionRef::Any => None,
            FunctionRef::Body(id) => {
                let data = self.function(id);
                let parameters = data
                    .parameters(method)
                    .iter()
                    .map(|parameter| {
                        let doc = data.doc.as_ref().and_then(|doc| doc.param(&parameter.name));

                        SignatureParameter {
                            name: &parameter.name,
                            ty: SignatureType::Value(self.slot_type(parameter.slot)),
                            default: parameter.default.as_deref().map(ParameterDefault::Source),
                            optional: doc.is_some_and(|doc| doc.optional),
                            is_annotated: doc.is_some_and(|doc| doc.ty.is_some()),
                            description: doc.and_then(|doc| doc.description.as_deref()),
                        }
                    })
                    .collect();

                let returns = match data.doc.as_ref().and_then(|doc| doc.returns.as_ref()) {
                    Some(guard @ TypeExpr::Guard { .. }) => SignatureType::Guard(guard),
                    _ => SignatureType::Value(self.slot_type(data.return_slot)),
                };

                Some(FunctionSignature {
                    parameters,
                    returns,
                })
            }
            FunctionRef::Signature(id) => {
                let declared_function = self.declared_function(id);
                let parameters = declared_function
                    .parameters
                    .iter()
                    .map(|parameter| SignatureParameter {
                        name: &parameter.name,
                        ty: SignatureType::Value(parameter.ty),
                        default: None,
                        optional: false,
                        is_annotated: true,
                        description: parameter.description.as_deref(),
                    })
                    .collect();

                Some(FunctionSignature {
                    parameters,
                    returns: SignatureType::Value(declared_function.return_type),
                })
            }
            FunctionRef::Api(api_function) => {
                let function = api_function.function();
                let parameters = function
                    .arguments()
                    .iter()
                    .map(|argument| {
                        Self::api_parameter(argument, SignatureType::Api(argument.types()))
                    })
                    .collect();
                let returns = match self.singleton_type(api_function) {
                    Some(ty) => SignatureType::Value(ty),
                    None => SignatureType::Api(function.returns()),
                };

                Some(FunctionSignature {
                    parameters,
                    returns,
                })
            }
            FunctionRef::ApiMethod(id) => {
                let method = self.api_method(id);
                let parameters = method
                    .function
                    .function()
                    .arguments()
                    .iter()
                    .zip(&method.parameters)
                    .map(|(argument, &ty)| Self::api_parameter(argument, SignatureType::Value(ty)))
                    .collect();

                Some(FunctionSignature {
                    parameters,
                    returns: SignatureType::Value(method.returns),
                })
            }
        }
    }

    pub(crate) fn parameter_count(&self, function: FunctionRef, method: bool) -> Option<usize> {
        Some(match function {
            FunctionRef::Any => return None,
            FunctionRef::Body(id) => self.function(id).parameters(method).len(),
            FunctionRef::Signature(id) => self.declared_function(id).parameters.len(),
            FunctionRef::Api(function) => function.function().arguments().len(),
            FunctionRef::ApiMethod(id) => {
                let method = self.api_method(id);

                method
                    .function
                    .function()
                    .arguments()
                    .len()
                    .min(method.parameters.len())
            }
        })
    }

    fn api_parameter<'a>(argument: &'a Argument, ty: SignatureType<'a>) -> SignatureParameter<'a> {
        SignatureParameter {
            name: argument.label(),
            ty,
            default: argument.default().map(ParameterDefault::Api),
            optional: argument.is_optional(),
            is_annotated: false,
            description: None,
        }
    }

    /// The `@deprecated` text of a script function in `ty`, empty when the tag has none.
    pub(crate) fn deprecation(&self, types: &TypeTable, ty: TypeId) -> Option<&str> {
        types.function_refs(ty).find_map(|function| match function {
            FunctionRef::Body(id) => self.function(id).doc.as_ref()?.deprecation.as_deref(),
            _ => None,
        })
    }

    pub(crate) fn documentation(&self, api: Api, function: FunctionRef) -> Option<Documentation> {
        match function {
            FunctionRef::Any => None,
            FunctionRef::Body(id) => {
                let doc = self.function(id).doc.as_ref()?;
                let description = match (doc.deprecation.as_deref(), doc.description.as_deref()) {
                    (None, description) => description.map(str::to_owned),
                    (Some(deprecation), description) => {
                        let mut text = if deprecation.is_empty() {
                            "Deprecated.".to_owned()
                        } else {
                            format!("Deprecated: {deprecation}")
                        };

                        if let Some(description) = description {
                            text.push_str("\n\n");
                            text.push_str(description);
                        }

                        Some(text)
                    }
                };

                Some(Documentation {
                    description,
                    examples: doc.examples.clone(),
                })
            }
            FunctionRef::Signature(id) => {
                let declared_function = self.declared_function(id);

                Some(Documentation {
                    description: declared_function.description.clone(),
                    examples: declared_function.examples.clone(),
                })
            }
            FunctionRef::ApiMethod(id) => {
                self.documentation(api, FunctionRef::Api(self.api_method(id).function))
            }
            FunctionRef::Api(function) => {
                let definition = api.definition(function)?;

                Some(Documentation {
                    description: definition.description().map(str::to_owned),
                    examples: definition
                        .example()
                        .map(<[String]>::to_vec)
                        .unwrap_or_default(),
                })
            }
        }
    }
}

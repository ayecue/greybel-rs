use rustc_hash::FxHashMap;

use super::Inference;
use crate::annotation::TemplateDoc;
use crate::model::{DeclaredId, MapOrigin};
use crate::types::{FunctionRef, Type, TypeId};

pub(super) type BindingTable = FxHashMap<Box<str>, TypeId>;

pub(super) type Evidence = FxHashMap<Box<str>, Vec<TypeId>>;

impl Inference<'_> {
    pub(super) fn type_parameters(&mut self, names: &[String]) -> Vec<(String, TypeId)> {
        names
            .iter()
            .map(|name| {
                let ty = self.types.insert(Type::TypeParameter {
                    name: name.as_str().into(),
                    bound: TypeId::ANY,
                });

                (name.clone(), ty)
            })
            .collect()
    }

    pub(super) fn template_parameters(
        &mut self,
        templates: &[TemplateDoc],
    ) -> Vec<(String, TypeId)> {
        templates
            .iter()
            .map(|template| {
                let bound = template
                    .bound
                    .as_ref()
                    .map_or(TypeId::ANY, |bound| self.resolve_type(bound));
                let ty = self.types.insert(Type::TypeParameter {
                    name: template.name.as_str().into(),
                    bound,
                });

                (template.name.clone(), ty)
            })
            .collect()
    }

    pub(super) fn has_type_parameters(&self, ty: TypeId) -> bool {
        match self.types.get(ty) {
            Type::TypeParameter { .. } => true,
            Type::List(item) => self.has_type_parameters(*item),
            Type::Map { key, value } => {
                self.has_type_parameters(*key) || self.has_type_parameters(*value)
            }
            Type::Union(variants) => variants
                .iter()
                .any(|&variant| self.has_type_parameters(variant)),
            Type::Object(_) => {
                self.model
                    .generic_instance_of(&self.types, ty)
                    .is_some_and(|specialization| {
                        specialization
                            .arguments
                            .iter()
                            .any(|&argument| self.has_type_parameters(argument))
                    })
            }
            Type::Function(FunctionRef::Signature(id)) => {
                let declared_function = self.model.declared_function(*id);

                self.has_type_parameters(declared_function.return_type)
                    || declared_function
                        .parameters
                        .iter()
                        .any(|parameter| self.has_type_parameters(parameter.ty))
            }
            _ => false,
        }
    }

    pub(super) fn substitute(&mut self, ty: TypeId, bindings: &BindingTable) -> TypeId {
        if !self.has_type_parameters(ty) {
            return ty;
        }

        match self.types.get(ty).clone() {
            Type::TypeParameter { name, .. } => bindings.get(&name).copied().unwrap_or(ty),
            Type::List(item) => {
                let item = self.substitute(item, bindings);

                self.types.list(item)
            }
            Type::Map { key, value } => {
                let key = self.substitute(key, bindings);
                let value = self.substitute(value, bindings);

                self.types.map(key, value)
            }
            Type::Union(variants) => {
                let variants: Vec<TypeId> = variants
                    .iter()
                    .map(|&variant| self.substitute(variant, bindings))
                    .collect();

                self.types.union_all(variants)
            }
            Type::Object(_) => {
                let Some(specialization) = self.model.generic_instance_of(&self.types, ty).cloned()
                else {
                    return ty;
                };
                let arguments = specialization
                    .arguments
                    .iter()
                    .map(|&argument| self.substitute(argument, bindings))
                    .collect();

                self.instantiate(specialization.generic, arguments)
            }
            Type::Function(FunctionRef::Signature(id)) => {
                let function = self.substitute_function(id, bindings);

                self.types.function(FunctionRef::Signature(function))
            }
            _ => ty,
        }
    }

    pub(super) fn bind(&mut self, parameter: TypeId, argument: TypeId, evidence: &mut Evidence) {
        if argument == TypeId::UNKNOWN || !self.has_type_parameters(parameter) {
            return;
        }

        if argument == TypeId::ANY {
            let mut names = Vec::new();

            self.type_parameter_names(parameter, &mut names);
            for name in names {
                evidence.entry(name).or_default().push(TypeId::ANY);
            }

            return;
        }

        let arguments: Vec<TypeId> = self.types.variants(argument).collect();

        match self.types.get(parameter).clone() {
            Type::TypeParameter { name, .. } => evidence.entry(name).or_default().push(argument),
            Type::Union(variants) => {
                for variant in variants {
                    self.bind(variant, argument, evidence);
                }
            }
            Type::List(item) => {
                for argument in arguments {
                    if let Type::List(argument) = *self.types.get(argument) {
                        self.bind(item, argument, evidence);
                    }
                }
            }
            Type::Map { key, value } => {
                for argument in arguments {
                    let (argument_key, argument_value) = match *self.types.get(argument) {
                        Type::Map { key, value } => (key, value),
                        Type::Object(map) => self.entry_types(map),
                        _ => continue,
                    };

                    self.bind(key, argument_key, evidence);
                    self.bind(value, argument_value, evidence);
                }
            }
            Type::Object(_) => {
                let Some(specialization) = self
                    .model
                    .generic_instance_of(&self.types, parameter)
                    .cloned()
                else {
                    return;
                };

                for argument in arguments {
                    let Some(argument_specialization) = self
                        .model
                        .generic_instance_of(&self.types, argument)
                        .cloned()
                    else {
                        continue;
                    };

                    if argument_specialization.generic != specialization.generic {
                        continue;
                    }

                    for (&parameter, &argument) in specialization
                        .arguments
                        .iter()
                        .zip(&argument_specialization.arguments)
                    {
                        self.bind(parameter, argument, evidence);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn type_parameter_bounds(&self, ty: TypeId, bounds: &mut Vec<(Box<str>, TypeId)>) {
        match self.types.get(ty) {
            Type::TypeParameter { name, bound } => {
                if !bounds.iter().any(|(it, _)| it == name) {
                    bounds.push((name.clone(), *bound));
                }
            }
            Type::List(item) => self.type_parameter_bounds(*item, bounds),
            Type::Map { key, value } => {
                self.type_parameter_bounds(*key, bounds);
                self.type_parameter_bounds(*value, bounds);
            }
            Type::Union(variants) => {
                for &variant in variants {
                    self.type_parameter_bounds(variant, bounds);
                }
            }
            _ => {}
        }
    }

    pub(super) fn type_parameter_names(&self, ty: TypeId, names: &mut Vec<Box<str>>) {
        match self.types.get(ty) {
            Type::TypeParameter { name, .. } => {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            Type::List(item) => self.type_parameter_names(*item, names),
            Type::Map { key, value } => {
                self.type_parameter_names(*key, names);
                self.type_parameter_names(*value, names);
            }
            Type::Union(variants) => {
                for &variant in variants {
                    self.type_parameter_names(variant, names);
                }
            }
            Type::Object(map) => {
                if let MapOrigin::GenericInstance(id) = self.model.map(*map).origin {
                    for &argument in &self.model.generic_instance(id).arguments {
                        self.type_parameter_names(argument, names);
                    }
                }
            }
            _ => {}
        }
    }

    fn substitute_function(&mut self, id: DeclaredId, bindings: &BindingTable) -> DeclaredId {
        let mut declared_function = self.model.declared_function(id).clone();

        declared_function.generic_source = Some(declared_function.generic_source.unwrap_or(id));
        for parameter in &mut declared_function.parameters {
            parameter.ty = self.substitute(parameter.ty, bindings);
        }

        declared_function.return_type = self.substitute(declared_function.return_type, bindings);

        self.model.insert_declared_instance(id, declared_function)
    }
}

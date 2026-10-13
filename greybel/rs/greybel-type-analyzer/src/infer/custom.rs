use greybel_core::{NodeId, Span};

use super::Inference;
use crate::annotation::{PropertyDoc, TypeDoc, TypeExpr};
use crate::model::{
    CustomType, DeclaredFunction, DeclaredId, DeclaredParameter, DocumentIndex, GenericInstance,
    GenericInstanceId, Key, KeyRef, MapId, MapOrigin,
};
use crate::types::{FunctionRef, Type, TypeId};

pub(super) struct TypeDeclaration {
    pub(super) document: DocumentIndex,
    pub(super) span: Span,
    pub(super) map: MapId,
    doc: TypeDoc,
    functions: Vec<DeclaredId>,
}

impl Inference<'_> {
    pub(super) fn declare_types(&mut self) {
        for document in 0..self.documents.len() {
            let docs: Vec<(Span, TypeDoc)> = self.documents[document]
                .comments
                .blocks
                .iter()
                .flat_map(|block| {
                    block
                        .doc
                        .types
                        .iter()
                        .map(|doc| (block.spans[doc.line], doc.clone()))
                })
                .collect();

            for (span, doc) in docs {
                if !doc.virtual_type && self.api.class_name(&doc.name).is_some() {
                    continue;
                }

                let map = match self.model.custom_type(&doc.name) {
                    Some(ty) => ty.map,
                    None => {
                        let name = self.model.insert_name(&doc.name);
                        let map = self.model.insert_map(MapOrigin::Custom(name));

                        self.model.add_custom_type(CustomType {
                            name: doc.name.clone(),
                            map,
                            document: document as DocumentIndex,
                            type_parameters: doc.type_parameters.clone(),
                            description: doc.description.clone(),
                        });

                        map
                    }
                };
                let functions = doc
                    .functions
                    .iter()
                    .map(|function| {
                        self.model.add_declared_function(DeclaredFunction {
                            name: function.name.clone(),
                            parameters: Vec::new(),
                            return_type: TypeId::UNKNOWN,
                            mutated_parameter: function.mutated_parameter.clone(),
                            generic_source: None,
                            description: function.description.clone(),
                            examples: function.examples.clone(),
                        })
                    })
                    .collect();

                self.type_declarations.push(TypeDeclaration {
                    document: document as DocumentIndex,
                    span,
                    map,
                    doc,
                    functions,
                });
            }
        }
    }

    pub(super) fn define_types(&mut self) {
        let declarations = std::mem::take(&mut self.type_declarations);

        for declaration in &declarations {
            let previous = (self.current_document, self.current_scope);

            self.current_document = declaration.document;
            self.current_scope = self.top_level_scopes[declaration.document as usize];

            let depth = self.type_parameters_in_scope.len();
            let type_parameters = self.type_parameters(&declaration.doc.type_parameters);

            self.type_parameters_in_scope.extend(type_parameters);
            self.define_type(declaration);
            self.type_parameters_in_scope.truncate(depth);
            (self.current_document, self.current_scope) = previous;
        }

        self.type_declarations = declarations;
    }

    fn define_type(&mut self, declaration: &TypeDeclaration) {
        let doc = &declaration.doc;
        let map = declaration.map;

        if let Some(extends) = &doc.extends {
            let parent = self.resolve_type(extends);
            let slot = self.model.map(map).isa;

            self.widen(slot, parent);
        }

        for property in &doc.properties {
            let has_children = doc.properties.iter().any(|other| {
                other.path.len() > property.path.len() && other.path.starts_with(&property.path)
            });

            self.define_property(map, property, has_children);
        }

        for (function, &id) in doc.functions.iter().zip(&declaration.functions) {
            let parameters = function
                .params
                .iter()
                .map(|param| DeclaredParameter {
                    name: param.name.clone(),
                    ty: param
                        .ty
                        .as_ref()
                        .map_or(TypeId::ANY, |ty| self.resolve_type(ty)),
                    description: param.description.clone(),
                })
                .collect();
            let returns = function
                .returns
                .as_ref()
                .map_or(TypeId::ANY, |ty| self.resolve_type(ty));
            let declared_function = self.model.declared_function_mut(id);

            declared_function.parameters = parameters;
            declared_function.return_type = returns;

            let value = self.types.function(FunctionRef::Signature(id));
            let slot = self.model.insert_entry(map, KeyRef::String(&function.name));

            self.widen(slot, value);
        }
    }

    fn define_property(&mut self, map: MapId, property: &PropertyDoc, has_children: bool) {
        let mut owner = map;
        let (last, path) = property.path.split_last().expect("properties have a name");

        for segment in path {
            let child_map = self.insert_nested_map(owner, segment);
            let object = self.object_type(child_map);
            let slot = self.model.insert_entry(owner, KeyRef::String(segment));

            self.widen(slot, object);
            owner = child_map;
        }

        let mut ty = self.resolve_type(&property.ty);

        if has_children {
            let variants: Vec<TypeId> = self
                .types
                .variants(ty)
                .filter(|&variant| !matches!(self.types.get(variant), Type::Map { .. }))
                .collect();

            ty = self.types.union_all(variants);

            let child_map = self.insert_nested_map(owner, last);
            let object = self.object_type(child_map);

            ty = self.types.union(ty, object);
        }

        let slot = self.model.insert_entry(owner, KeyRef::String(last));

        self.widen(slot, ty);
    }

    fn insert_nested_map(&mut self, owner: MapId, name: &str) -> MapId {
        let name = self.model.insert_name(name);

        self.model.insert_map(MapOrigin::NestedKey { owner, name })
    }

    pub(super) fn custom_type(&mut self, name: &str, arguments: Vec<TypeId>) -> Option<TypeId> {
        let map = self.model.custom_type(name)?.map;

        if arguments.is_empty() {
            return Some(self.object_type(map));
        }

        Some(self.instantiate(map, arguments))
    }

    pub(super) fn instantiate(&mut self, generic: MapId, arguments: Vec<TypeId>) -> TypeId {
        let id = self
            .model
            .insert_generic_instance(GenericInstance { generic, arguments });
        let map = self.model.insert_map(MapOrigin::GenericInstance(id));

        if self.pass.generic_instance_set.insert(id) {
            self.fill_instance(id, map);
        }

        self.object_type(map)
    }

    fn fill_instance(&mut self, id: GenericInstanceId, map: MapId) {
        let GenericInstance { generic, arguments } = self.model.generic_instance(id).clone();
        let type_parameters = self
            .model
            .custom_type_of(generic)
            .map(|ty| ty.type_parameters.clone())
            .unwrap_or_default();
        let bindings = type_parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                (
                    parameter.as_str().into(),
                    arguments.get(index).copied().unwrap_or(TypeId::ANY),
                )
            })
            .collect();
        let data = self.model.map(generic);
        let entries: Vec<(Key, TypeId)> = data
            .entries
            .iter()
            .map(|(key, &slot)| (key.clone(), self.model.slot_type(slot)))
            .collect();
        let parent = self.model.slot_type(data.isa);

        for (key, ty) in entries {
            let ty = self.substitute(ty, &bindings);
            let slot = self.model.insert_entry(map, key.as_key_ref());

            self.widen(slot, ty);
        }

        let parent = self.substitute(parent, &bindings);
        let slot = self.model.map(map).isa;

        self.widen(slot, parent);
    }

    pub(super) fn custom_type_at(&mut self, node: NodeId) -> Option<TypeId> {
        let name = self
            .doc_at(node)?
            .types
            .iter()
            .find(|ty| !ty.virtual_type)?
            .name
            .clone();

        if let Some(ty) = self.model.custom_type(&name) {
            let map = ty.map;

            return Some(self.object_type(map));
        }

        Some(self.resolve_type(&TypeExpr::Name {
            name,
            arguments: Vec::new(),
        }))
    }
}

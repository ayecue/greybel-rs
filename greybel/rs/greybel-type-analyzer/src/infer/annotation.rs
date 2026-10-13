use greybel_core::NodeId;
use greybel_symbols::annotations::{ANY, BOOL, BOOLEAN, NIL, UNKNOWN};
use greybel_symbols::names::SELF;
use greybel_symbols::types;

use super::Inference;
use crate::annotation::{DocComment, TypeExpr};
use crate::types::{FunctionRef, Type, TypeId};

impl Inference<'_> {
    pub(crate) fn doc_at(&self, node: NodeId) -> Option<&DocComment> {
        let ast = self.ast();
        let line = ast.lines().line(ast.span(node).start);

        self.documents[self.current_document as usize]
            .comments
            .doc_at(line)
    }

    pub(crate) fn annotated_type_at(&mut self, node: NodeId) -> Option<TypeId> {
        let ty = match self.doc_at(node)?.defined_type.clone() {
            Some(define) => self.resolve_type(&define),
            None => self.custom_type_at(node)?,
        };

        (ty != TypeId::UNKNOWN).then_some(ty)
    }

    pub(crate) fn resolve_type(&mut self, expr: &TypeExpr) -> TypeId {
        match expr {
            TypeExpr::Union(variants) => {
                let variants: Vec<TypeId> = variants
                    .iter()
                    .map(|variant| self.resolve_type(variant))
                    .collect();

                self.types.union_all(variants)
            }
            TypeExpr::Name { name, arguments } => {
                let arguments: Vec<TypeId> = arguments
                    .iter()
                    .map(|argument| self.resolve_type(argument))
                    .collect();
                let argument = |index: usize| arguments.get(index).copied().unwrap_or(TypeId::ANY);

                match name.as_str() {
                    types::NUMBER | BOOL | BOOLEAN => TypeId::NUMBER,
                    types::STRING => TypeId::STRING,
                    types::NULL | NIL => TypeId::NULL,
                    ANY => TypeId::ANY,
                    UNKNOWN => TypeId::UNKNOWN,
                    SELF => self.types.type_parameter(SELF),
                    types::FUNCTION | types::FUNC_REF => self.types.function(FunctionRef::Any),
                    types::LIST => self.types.list(argument(0)),
                    types::MAP => self.types.map(argument(0), argument(1)),
                    _ => self.resolve_type_name(name, arguments),
                }
            }
        }
    }

    fn resolve_type_name(&mut self, name: &str, arguments: Vec<TypeId>) -> TypeId {
        if let Some(&(_, parameter)) = self
            .type_parameters_in_scope
            .iter()
            .rev()
            .find(|(generic, _)| generic == name)
        {
            return parameter;
        }

        if let Some(ty) = self.custom_type(name, arguments) {
            return ty;
        }

        if let Some(class) = self.api.class_name(name) {
            return self.types.insert(Type::Class(class));
        }

        if let Some(slot) = self.resolve_variable(name) {
            let ty = self.model.slot_type(slot);

            if ty == TypeId::UNKNOWN {
                return TypeId::UNKNOWN;
            }

            let objects: Vec<TypeId> = self
                .types
                .variants(ty)
                .filter(|&variant| matches!(self.types.get(variant), Type::Object(_)))
                .collect();

            return if objects.is_empty() {
                TypeId::ANY
            } else {
                self.types.union_all(objects)
            };
        }

        if self.implicit_generics {
            return self.types.insert(Type::TypeParameter {
                name: name.into(),
                bound: TypeId::ANY,
            });
        }

        TypeId::ANY
    }
}

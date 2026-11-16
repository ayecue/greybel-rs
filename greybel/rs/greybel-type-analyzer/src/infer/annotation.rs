use std::sync::Arc;

use greybel_core::{NodeId, NodeKind, Span};
use greybel_symbols::annotations::{ANY, BOOL, BOOLEAN, NEVER, NIL, TAG_PREFIX, Tag, UNKNOWN};
use greybel_symbols::names::SELF;
use greybel_symbols::types;

use super::Inference;
use crate::annotation::{DocComment, ParamDoc, TemplateDoc, TextPosition, TypeExpr, TypeLiteral};
use crate::diagnostic::DiagnosticKind;
use crate::types::{FunctionRef, LiteralType, Type, TypeId};

impl Inference<'_> {
    pub(crate) fn doc_at(&self, node: NodeId) -> Option<&Arc<DocComment>> {
        let ast = self.ast();
        let line = ast.lines().line(ast.span(node).start);

        self.documents[self.current_document as usize]
            .comments
            .doc_at(line)
    }

    /// The type written in code at `span`, as a type expression; reports one that isn't one.
    pub(crate) fn written_type_expr(&mut self, span: Span) -> Option<TypeExpr> {
        let text = self.ast().slice(span);
        let expr = TypeExpr::parse(text);

        if expr.is_none() {
            self.report_span(
                self.current_document,
                DiagnosticKind::InvalidAnnotation,
                span,
                format!("Cannot read the type \"{text}\"."),
            );
        }

        expr
    }

    /// The documentation of `function` with the types written in its code (parameter types, the return type and
    /// type parameters) added: a type in code counts like the same type in a doc comment, and replaces one written
    /// both ways (see [`Self::validate_written_types`]).
    pub(crate) fn with_written_types(
        &mut self,
        function: NodeId,
        comment: Option<Arc<DocComment>>,
    ) -> Option<Arc<DocComment>> {
        let ast = self.ast();
        let NodeKind::Function(declaration) = ast.kind(function) else {
            return comment;
        };
        let types = &ast.type_syntax;
        let parameters: Vec<(String, Span)> = ast
            .list(declaration.parameters)
            .iter()
            .filter_map(|&parameter| {
                let variable = match ast.kind(parameter) {
                    NodeKind::Assignment(assignment) => assignment.variable,
                    _ => parameter,
                };
                let span = types.parameter(variable)?;

                Some((ast.text(variable)?.to_owned(), span))
            })
            .collect();
        let returns = types.returns(function);
        let templates: Vec<String> = types
            .templates(function)
            .iter()
            .map(|&name| ast.slice(name).to_owned())
            .collect();

        if parameters.is_empty() && returns.is_none() && templates.is_empty() {
            return comment;
        }

        let mut doc = comment.as_deref().cloned().unwrap_or_default();

        for (name, span) in parameters {
            let Some(ty) = TypeExpr::parse(ast.slice(span)) else {
                continue;
            };

            match doc.params.iter_mut().find(|param| param.name == name) {
                Some(param) => param.ty = Some(ty),
                None => doc.params.push(ParamDoc {
                    name,
                    name_position: TextPosition::default(),
                    ty: Some(ty),
                    description: None,
                    optional: false,
                }),
            }
        }

        if let Some(ty) = returns.and_then(|span| TypeExpr::parse(ast.slice(span))) {
            doc.returns = Some(ty);
        }

        for name in templates {
            if !doc.templates.iter().any(|template| template.name == name) {
                doc.templates.push(TemplateDoc { name, bound: None });
            }
        }

        Some(Arc::new(doc))
    }

    /// Reports the types written in the code of `function` that aren't types, and parameter and return types that
    /// are written both in code and in its doc comment.
    pub(crate) fn validate_written_types(&mut self, function: NodeId) {
        if self.context.current.is_some() {
            return;
        }

        let ast = self.ast();
        let NodeKind::Function(declaration) = ast.kind(function) else {
            return;
        };
        let types = &ast.type_syntax;
        let comment = declaration
            .assignment
            .and_then(|assignment| self.doc_at(assignment))
            .map(Arc::clone);

        for &parameter in ast.list(declaration.parameters) {
            let variable = match ast.kind(parameter) {
                NodeKind::Assignment(assignment) => assignment.variable,
                _ => parameter,
            };
            let Some(span) = types.parameter(variable) else {
                continue;
            };
            let name = ast.text(variable).unwrap_or_default();

            self.written_type_expr(span);

            let documented = comment.as_ref().is_some_and(|doc| {
                doc.params
                    .iter()
                    .any(|param| param.name == name && param.ty.is_some())
            });

            if documented {
                self.report_span(
                    self.current_document,
                    DiagnosticKind::InvalidAnnotation,
                    span,
                    format!(
                        "\"{name}\" has a type in code and in {TAG_PREFIX}{}.",
                        Tag::Param
                    ),
                );
            }
        }

        if let Some(span) = types.returns(function) {
            self.written_type_expr(span);

            if comment.as_ref().is_some_and(|doc| doc.returns.is_some()) {
                self.report_span(
                    self.current_document,
                    DiagnosticKind::InvalidAnnotation,
                    span,
                    format!(
                        "The return type is in code and in {TAG_PREFIX}{}.",
                        Tag::Returns
                    ),
                );
            }
        }
    }

    pub(crate) fn annotated_type_at(&mut self, node: NodeId) -> Option<TypeId> {
        let ast = self.ast();

        if let NodeKind::Assignment(assignment) = ast.kind(node)
            && let Some(span) = ast.type_syntax.variable(assignment.variable)
        {
            let expr = self.written_type_expr(span)?;
            let ty = self.resolve_type(&expr);

            return (ty != TypeId::UNKNOWN).then_some(ty);
        }

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
            TypeExpr::Literal(TypeLiteral::String(text)) => self
                .types
                .insert(Type::Literal(LiteralType::String(text.as_str().into()))),
            TypeExpr::Literal(TypeLiteral::Number(text)) => {
                let value = text.parse().unwrap_or_default();

                self.types.insert(Type::Literal(LiteralType::number(value)))
            }
            TypeExpr::Tuple(elements) => {
                let elements: Vec<TypeId> = elements
                    .iter()
                    .map(|element| self.resolve_type(element))
                    .collect();
                let item = self.types.union_all(elements);

                self.types.list(item)
            }
            TypeExpr::Guard { .. } => TypeId::NUMBER,
            TypeExpr::Function {
                parameters,
                returns,
            } => {
                let parameters: Vec<(String, TypeId)> = parameters
                    .iter()
                    .enumerate()
                    .map(|(index, parameter)| {
                        let name = parameter
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("arg{}", index + 1));

                        (name, self.resolve_type(&parameter.ty))
                    })
                    .collect();
                let return_type = match returns {
                    Some(returns) => self.resolve_type(returns),
                    None => TypeId::ANY,
                };
                let signature = self.model.insert_signature(parameters, return_type);

                self.types.function(FunctionRef::Signature(signature))
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
                    NEVER => TypeId::NEVER,
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

        if let Some(ty) = self.alias_type(name, &arguments) {
            return ty;
        }

        if let Some(ty) = self.custom_type(name, arguments) {
            return ty;
        }

        if let Some(&ty) = self.enum_types.get(name) {
            return ty;
        }

        if let Some(class) = self.api.class_name(name) {
            return self.types.insert(Type::Class(class));
        }

        if let Some(slot) = self.model.resolve_variable(self.current_scope, name) {
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

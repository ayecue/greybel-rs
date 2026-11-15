use greybel_core::ast::NodeFieldContainer;
use greybel_core::{Ast, NodeId, NodeKind};
use greybel_symbols::Keyword;
use greybel_symbols::annotations::{TAG_PREFIX, Tag};
use greybel_symbols::names::SELF;
use rustc_hash::FxHashSet;

use super::Inference;
use super::generic::Evidence;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::model::{DocumentIndex, ScopeId, SignatureType};
use crate::types::{FunctionRef, Type, TypeId, TypeNotation};

pub(super) struct DeferredDiagnostic {
    document: DocumentIndex,
    kind: DiagnosticKind,
    node: NodeId,
    message: Vec<MessagePart>,
}

enum MessagePart {
    Text(String),
    Type(TypeId),
}

impl DeferredDiagnostic {
    pub(super) fn document(&self) -> DocumentIndex {
        self.document
    }

    pub(super) fn render(&self, notation: TypeNotation<'_>, ast: &Ast) -> Diagnostic {
        let message = self
            .message
            .iter()
            .map(|part| match part {
                MessagePart::Text(text) => text.clone(),
                MessagePart::Type(ty) => notation.display(*ty).to_string(),
            })
            .collect();

        Diagnostic::new(self.kind, ast.lines(), ast.span(self.node), message)
    }
}

impl Inference<'_> {
    fn defer_diagnostic(&mut self, kind: DiagnosticKind, node: NodeId, message: Vec<MessagePart>) {
        if self.call_context.is_none() {
            self.pass.deferred_diagnostics.push(DeferredDiagnostic {
                document: self.current_document,
                kind,
                node,
                message,
            });
        }
    }

    fn is_assignable(&self, actual: TypeId, target: TypeId) -> bool {
        self.types.variants(actual).any(|actual| {
            self.types
                .variants(target)
                .any(|target| self.is_variant_assignable(actual, target))
        })
    }

    fn is_variant_assignable(&self, actual: TypeId, target: TypeId) -> bool {
        let lenient =
            |ty: &Type| matches!(ty, Type::Any | Type::Unknown | Type::TypeParameter { .. });
        let (actual, target) = (self.types.get(actual), self.types.get(target));

        if lenient(actual) || lenient(target) || matches!(actual, Type::Null) {
            return true;
        }

        match (actual, target) {
            (Type::Class(actual), Type::Class(target)) => self.api.inherits(actual, target),
            (
                Type::Map { .. } | Type::Object(_) | Type::Class(_),
                Type::Map { .. } | Type::Object(_) | Type::Class(_),
            )
            | (Type::Number, Type::Number)
            | (Type::String, Type::String)
            | (Type::List(_), Type::List(_))
            | (Type::Function(_), Type::Function(_)) => true,
            _ => false,
        }
    }

    fn is_never_callable(&self, ty: TypeId) -> bool {
        self.types.variants(ty).all(|variant| {
            !matches!(
                self.types.get(variant),
                Type::Function(_) | Type::Any | Type::Unknown | Type::TypeParameter { .. }
            )
        })
    }

    pub(crate) fn validate_call(
        &mut self,
        node: NodeId,
        callee: TypeId,
        arguments: &[TypeId],
        method: bool,
    ) {
        if callee == TypeId::UNKNOWN {
            return;
        }

        if !arguments.is_empty() && self.is_never_callable(callee) {
            self.defer_diagnostic(
                DiagnosticKind::CallOfNonCallable,
                node,
                vec![
                    MessagePart::Text("Cannot call non-callable type ".into()),
                    MessagePart::Type(callee),
                    MessagePart::Text(".".into()),
                ],
            );

            return;
        }

        let functions: Vec<FunctionRef> = self.types.function_refs(callee).collect();

        if functions.len() != self.types.variants(callee).count() {
            return;
        }

        let mut max_parameters = None;

        for &function in &functions {
            let Some(signature) = self.model.signature(function, method) else {
                return;
            };

            max_parameters = max_parameters.max(Some(signature.parameters.len()));
        }

        if let Some(max_parameters) =
            max_parameters.filter(|&max_parameters| arguments.len() > max_parameters)
        {
            self.report(
                DiagnosticKind::CallArityMismatch,
                node,
                format!(
                    "Call expected at most {max_parameters} arguments but got {}.",
                    arguments.len()
                ),
            );

            return;
        }

        if let [function] = functions[..] {
            self.validate_arguments(node, function, arguments, method);
        }
    }

    fn validate_arguments(
        &mut self,
        node: NodeId,
        function: FunctionRef,
        arguments: &[TypeId],
        method: bool,
    ) {
        let Some(signature) = self.model.signature(function, method) else {
            return;
        };
        let parameter_types: Vec<Option<TypeId>> = signature
            .parameters
            .iter()
            .map(|parameter| match parameter.ty {
                SignatureType::Value(ty) if parameter.is_annotated => Some(ty),
                _ => None,
            })
            .collect();

        self.validate_bounds(node, &parameter_types, arguments);
        for (index, (&parameter_type, &actual)) in parameter_types.iter().zip(arguments).enumerate()
        {
            let Some(parameter_type) = parameter_type else {
                continue;
            };

            if !self.is_assignable(actual, parameter_type) {
                self.defer_diagnostic(
                    DiagnosticKind::CallSignatureMismatch,
                    node,
                    vec![
                        MessagePart::Text(format!("Argument {} expected ", index + 1)),
                        MessagePart::Type(parameter_type),
                        MessagePart::Text(" but got ".into()),
                        MessagePart::Type(actual),
                    ],
                );
            }
        }
    }

    fn validate_bounds(
        &mut self,
        node: NodeId,
        parameter_types: &[Option<TypeId>],
        arguments: &[TypeId],
    ) {
        let mut evidence = Evidence::default();
        let mut bounds = Vec::new();

        for (&parameter_type, &actual) in parameter_types.iter().zip(arguments) {
            let Some(parameter_type) = parameter_type else {
                continue;
            };

            self.bind(parameter_type, actual, &mut evidence);
            self.type_parameter_bounds(parameter_type, &mut bounds);
        }

        for (name, bound) in bounds {
            if bound == TypeId::ANY || bound == TypeId::UNKNOWN {
                continue;
            }

            let Some(types) = evidence.get(&name).cloned() else {
                continue;
            };
            let actual = self.types.union_all(types);

            if !self.is_assignable(actual, bound) {
                self.defer_diagnostic(
                    DiagnosticKind::TypeMismatch,
                    node,
                    vec![
                        MessagePart::Text("Type argument ".into()),
                        MessagePart::Type(actual),
                        MessagePart::Text(" does not satisfy constraint ".into()),
                        MessagePart::Type(bound),
                        MessagePart::Text(format!(" for {name}.")),
                    ],
                );
            }
        }
    }

    pub(super) fn validate_index(&mut self, node: NodeId, base: TypeId) {
        let indexable = self.types.variants(base).any(|variant| {
            !matches!(
                self.types.get(variant),
                Type::Number | Type::Function(_) | Type::Null
            )
        });

        if !indexable && base != TypeId::UNKNOWN {
            self.defer_diagnostic(
                DiagnosticKind::IndexOfNonIndexable,
                node,
                vec![
                    MessagePart::Text("Cannot index non-indexable type ".into()),
                    MessagePart::Type(base),
                    MessagePart::Text(".".into()),
                ],
            );
        }
    }

    pub(super) fn validate_against_annotation(
        &mut self,
        node: NodeId,
        tag: Tag,
        name: &str,
        annotation_type: TypeId,
        inferred_type: TypeId,
    ) {
        if !self.is_assignable(inferred_type, annotation_type) {
            self.defer_diagnostic(
                DiagnosticKind::AnnotationSignatureMismatch,
                node,
                vec![
                    MessagePart::Text(format!("{TAG_PREFIX}{tag} {name} expects ")),
                    MessagePart::Type(annotation_type),
                    MessagePart::Text(" but inferred ".into()),
                    MessagePart::Type(inferred_type),
                    MessagePart::Text(".".into()),
                ],
            );
        }
    }

    pub(super) fn function_name(&self, function: NodeId) -> String {
        let ast = self.ast();
        let NodeKind::Function(declaration) = ast.kind(function) else {
            return Keyword::Function.as_str().to_owned();
        };

        declaration
            .assignment
            .and_then(|assignment| match ast.kind(assignment) {
                NodeKind::Assignment(assignment) => ast.text(assignment.variable),
                _ => None,
            })
            .unwrap_or(Keyword::Function.as_str())
            .to_owned()
    }

    pub(super) fn validate_variables(&mut self) {
        let read = self.references.read_slots();
        let mut unused_diagnostics = Vec::new();

        for id in self.model.functions() {
            let function = self.model.function(id);
            let document = function.site.document;
            let ast = self.documents[document as usize].ast;

            if !matches!(ast.kind(function.site.node), NodeKind::Function(_)) {
                continue;
            }

            let assignment_names: FxHashSet<&str> = ast
                .kind(function.site.node)
                .as_scope()
                .map(|scope| ast.list(scope.scope().definitions))
                .unwrap_or_default()
                .iter()
                .filter_map(|&definition| match ast.kind(definition) {
                    NodeKind::Assignment(assignment) => match ast.kind(assignment.variable) {
                        NodeKind::Identifier(identifier) => Some(ast.resolve(identifier.name)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();

            for (name, slot) in self.model.variables(ScopeId::Function(id)) {
                let Some(&first) = self
                    .references
                    .definition_sites(slot)
                    .iter()
                    .find(|site| site.document == document)
                else {
                    continue;
                };
                let parameter = function
                    .parameters
                    .iter()
                    .any(|parameter| parameter.name == name);

                if !parameter && !assignment_names.contains(name) {
                    continue;
                }

                let is_unused_reportable = name != SELF
                    && !name.starts_with('_')
                    && self
                        .model
                        .known_slot_type(slot)
                        .is_none_or(|ty| self.types.function_refs(ty).next().is_none());

                if is_unused_reportable && !read.contains(&slot) {
                    let (kind, label) = if parameter {
                        (DiagnosticKind::UnusedParameter, "Unused parameter")
                    } else {
                        (DiagnosticKind::UnusedLocal, "Unused local")
                    };

                    unused_diagnostics.push((kind, first, format!("{label}: {name}")));
                }

                if !parameter && self.has_outer_binding(function.outer_scope, name) {
                    unused_diagnostics.push((
                        DiagnosticKind::Shadow,
                        first,
                        format!("Shadowing outer binding: {name}"),
                    ));
                }
            }
        }

        for (kind, site, message) in unused_diagnostics {
            self.report_in(site.document, kind, site.node, message);
        }
    }

    fn has_outer_binding(&self, outer: ScopeId, name: &str) -> bool {
        [outer, ScopeId::Global]
            .into_iter()
            .any(|scope| self.model.variable(scope, name).is_some())
    }
}

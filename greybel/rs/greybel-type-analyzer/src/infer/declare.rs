use std::sync::Arc;

use greybel_core::ast::{NodeFieldContainer, Scope};
use greybel_core::{NodeId, NodeKind};
use greybel_documents::ImportKind;
use greybel_symbols::names::MODULE;
use rustc_hash::FxHashSet;

use super::Inference;
use crate::model::{DocumentIndex, FunctionId, Parameter, ScopeId, Site};

impl Inference<'_> {
    pub(super) fn declare(&mut self) {
        let mut document_stack = vec![0];
        let mut reachable = FxHashSet::from_iter([0]);

        while let Some(document) = document_stack.pop() {
            let mut dependencies: Vec<_> = self.documents[document as usize]
                .dependencies
                .values()
                .copied()
                .collect();

            dependencies.sort_unstable_by_key(|&(_, target)| target);
            for (kind, target) in dependencies {
                if !kind.runs_code() || !reachable.insert(target) {
                    continue;
                }

                self.top_level_scopes[target as usize] = match kind {
                    ImportKind::Import => self.module_scope(target),
                    _ => self.top_level_scopes[document as usize],
                };
                document_stack.push(target);
            }
        }

        self.declare_types();
        for document in reachable {
            self.current_document = document;
            self.declare_document();
        }

        self.current_document = 0;
    }

    fn declare_document(&mut self) {
        let ast = self.ast();

        for &function in &ast.scopes {
            self.function_id(function);
        }

        let top_level = self.top_level_scopes[self.current_document as usize];

        self.declare_variables(top_level, ast.root());
    }

    pub(super) fn declare_variables(&mut self, scope: ScopeId, node: NodeId) {
        let ast = self.ast();
        let Some(data) = ast.kind(node).as_scope().map(Scope::scope) else {
            return;
        };

        for &definition in ast.list(data.definitions) {
            let variable = match ast.kind(definition) {
                NodeKind::Assignment(assignment) => assignment.variable,
                NodeKind::For(for_loop) => for_loop.variable,
                _ => continue,
            };

            if let Some(name) = self.identifier_name_id(variable) {
                self.model.insert_variable_by_id(scope, name);
            }
        }
    }

    fn module_scope(&mut self, document: DocumentIndex) -> ScopeId {
        let site = Site {
            document,
            node: self.documents[document as usize].ast.root(),
        };
        let scope = ScopeId::Function(self.model.add_function(site, ScopeId::Global, None));

        self.model.insert_variable(scope, MODULE);

        scope
    }

    pub(crate) fn function_id(&mut self, node: NodeId) -> FunctionId {
        let site = self.site(node);

        if let Some(id) = self.model.function_in_context(site, self.context.current) {
            return id;
        }

        let ast = self.ast();
        let NodeKind::Function(function) = ast.kind(node) else {
            unreachable!("functions are function nodes")
        };
        let outer = match ast.node(node).scope {
            Some(scope) if scope != ast.root() => ScopeId::Function(self.function_id(scope)),
            _ => self.top_level_scopes[self.current_document as usize],
        };
        let id = self.model.add_function(site, outer, self.context.current);
        let scope = ScopeId::Function(id);
        let parameters = ast
            .list(function.parameters)
            .iter()
            .filter_map(|&parameter| {
                let (variable, default) = match ast.kind(parameter) {
                    NodeKind::Assignment(assignment) => (
                        assignment.variable,
                        ast.text(assignment.init).map(str::to_owned),
                    ),
                    _ => (parameter, None),
                };
                let name = self.identifier_name(variable)?.to_owned();

                Some(Parameter {
                    slot: self.model.insert_variable(scope, &name),
                    name,
                    node: parameter,
                    default,
                })
            })
            .collect();
        let comment = function
            .assignment
            .and_then(|assignment| self.doc_at(assignment))
            .map(Arc::clone);
        let doc = self.with_written_types(node, comment);
        let data = self.model.function_mut(id);

        data.parameters = parameters;
        data.doc = doc;
        self.declare_variables(scope, node);

        id
    }
}

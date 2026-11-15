mod annotation;
mod buffers;
mod call;
mod context;
mod custom;
mod directive;
mod expression;
mod flow;
mod generic;
mod inheritance;
mod member;
mod mutation;
mod narrow;
mod statement;
mod validation;

use buffers::TypeBufferPool;
use custom::TypeDeclaration;
use flow::GlobalRead;
use greybel_core::ast::{NodeFieldContainer, Scope};
use greybel_core::{Ast, NodeId, NodeKind, Span};
use greybel_documents::ImportKind;
use greybel_symbols::Keyword;
use greybel_symbols::names::{EXPORTS, MODULE};
use narrow::NarrowStack;
use rustc_hash::{FxHashMap, FxHashSet};
use validation::DeferredDiagnostic;

use crate::analysis::{Analysis, ReferenceTable};
use crate::annotation::CommentTable;
use crate::api::Api;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::lookup::Lookup;
use crate::model::{
    CallContextId, DocumentIndex, FunctionId, GenericInstanceId, KeyRef, MapId, MapOrigin,
    Parameter, ScopeId, SemanticModel, Site, SlotId,
};
use crate::types::{Type, TypeId, TypeNotation, TypeTable};

const MAX_PASSES: usize = 32;

pub(crate) struct Document<'a> {
    pub ast: &'a Ast,
    pub comments: &'a CommentTable,
    pub dependencies: FxHashMap<NodeId, (ImportKind, DocumentIndex)>,
}

pub(crate) struct Inference<'a> {
    api: Api,
    types: TypeTable,
    model: SemanticModel,
    references: ReferenceTable,
    documents: Vec<Document<'a>>,
    top_level_scopes: Vec<ScopeId>,
    type_declarations: Vec<TypeDeclaration>,
    node_types: Vec<Vec<Option<TypeId>>>,
    pass: Pass,
    current_document: DocumentIndex,
    current_scope: ScopeId,
    type_parameters_in_scope: Vec<(String, TypeId)>,
    implicit_generics: bool,
    custom_map: Option<(Site, MapId)>,
    narrow_stack: NarrowStack,
    loop_stack: Vec<Site>,
    call_context: Option<CallContextId>,
    context_depth: usize,
    context_height: usize,
    context_types: FxHashMap<Site, TypeId>,
    type_buffers: TypeBufferPool,
    context_references: Vec<ReferenceTable>,
}

#[derive(Default)]
struct Pass {
    has_changes: bool,
    visited_documents: FxHashSet<DocumentIndex>,
    inferred_contexts: FxHashSet<CallContextId>,
    generic_instance_set: FxHashSet<GenericInstanceId>,
    diagnostics: Vec<(DocumentIndex, Diagnostic)>,
    deferred_diagnostics: Vec<DeferredDiagnostic>,
    global_reads: Vec<GlobalRead>,
}

impl<'a> Inference<'a> {
    pub(crate) fn run(documents: Vec<Document<'a>>, api: Api) -> Analysis {
        let mut inference = Self::new(documents, api);

        inference.declare();
        for _ in 0..MAX_PASSES {
            inference.pass = Pass::default();
            inference.pass.visited_documents.insert(0);
            for node_types in &mut inference.node_types {
                node_types.fill(None);
            }

            inference.define_types();

            let body = inference.documents[0].ast.body();

            inference.infer_block(body);
            if !inference.pass.has_changes {
                break;
            }
        }

        inference.finish()
    }

    fn new(documents: Vec<Document<'a>>, api: Api) -> Self {
        Self {
            api,
            types: TypeTable::default(),
            model: SemanticModel::default(),
            references: ReferenceTable::new(
                documents.iter().map(|document| document.ast.nodes().len()),
            ),
            top_level_scopes: vec![ScopeId::Global; documents.len()],
            type_declarations: Vec::new(),
            node_types: documents
                .iter()
                .map(|document| vec![None; document.ast.nodes().len()])
                .collect(),
            documents,
            pass: Pass::default(),
            current_document: 0,
            current_scope: ScopeId::Global,
            type_parameters_in_scope: Vec::new(),
            implicit_generics: false,
            custom_map: None,
            narrow_stack: NarrowStack::default(),
            loop_stack: Vec::new(),
            call_context: None,
            context_depth: 0,
            context_height: 0,
            context_types: FxHashMap::default(),
            type_buffers: TypeBufferPool::default(),
            context_references: Vec::new(),
        }
    }

    fn finish(mut self) -> Analysis {
        self.validate_variables();
        self.validate_definition_order();
        self.validate_inheritance();
        for (document, source) in self.documents.iter().enumerate() {
            let (comments, lines) = (source.comments, source.ast.lines());

            for block in &comments.blocks {
                for tag in &block.doc.invalid_tags {
                    self.pass.diagnostics.push((
                        DocumentIndex::try_from(document)
                            .expect("value fits the target integer type"),
                        Diagnostic::new(
                            DiagnosticKind::InvalidAnnotation,
                            lines,
                            block.spans[tag.line],
                            tag.message.clone(),
                        ),
                    ));
                }
            }
        }

        let notation = TypeNotation::new(&self.model, &self.types);
        let deferred_entries: Vec<(DocumentIndex, Diagnostic)> = self
            .pass
            .deferred_diagnostics
            .iter()
            .map(|diagnostic| {
                let document = diagnostic.document();
                let ast = self.documents[document as usize].ast;

                (document, diagnostic.render(notation, ast))
            })
            .collect();
        let mut diagnostics = vec![Vec::new(); self.documents.len()];

        for (document, diagnostic) in self.pass.diagnostics.into_iter().chain(deferred_entries) {
            diagnostics[document as usize].push(diagnostic);
        }

        for list in &mut diagnostics {
            Diagnostic::sort(list);
        }

        Analysis::new(
            self.api,
            self.types,
            self.model,
            self.node_types,
            self.references,
            diagnostics,
        )
    }

    pub(crate) fn ast(&self) -> &'a Ast {
        self.documents[self.current_document as usize].ast
    }

    pub(crate) fn site(&self, node: NodeId) -> Site {
        Site {
            document: self.current_document,
            node,
        }
    }

    pub(crate) fn lookup(&mut self) -> Lookup<'_> {
        Lookup::new(&mut self.types, &self.model, self.api)
    }

    pub(crate) fn dependency(&self, node: NodeId) -> Option<(ImportKind, DocumentIndex)> {
        self.documents[self.current_document as usize]
            .dependencies
            .get(&node)
            .copied()
    }

    fn declare(&mut self) {
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
                if !reachable.insert(target) {
                    continue;
                }

                self.top_level_scopes[target as usize] = match kind {
                    ImportKind::Import => self.module_scope(target),
                    ImportKind::Include | ImportKind::NativeImport | ImportKind::Inject => {
                        self.top_level_scopes[document as usize]
                    }
                };
                document_stack.push(target);
            }
        }

        self.declare_types();
        for document in 0..self.documents.len() {
            self.current_document =
                DocumentIndex::try_from(document).expect("value fits the target integer type");
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

    fn declare_variables(&mut self, scope: ScopeId, node: NodeId) {
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

            if let Some(name) = self.identifier_name(variable) {
                self.model.insert_variable(scope, name);
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

        if let Some(id) = self.model.function_in_context(site, self.call_context) {
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
        let id = self.model.add_function(site, outer, self.call_context);
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
        let doc = function
            .assignment
            .and_then(|assignment| self.doc_at(assignment))
            .cloned();
        let data = self.model.function_mut(id);

        data.parameters = parameters;
        data.doc = doc;
        self.declare_variables(scope, node);

        id
    }

    pub(crate) fn infer_dependency(&mut self, node: NodeId) -> Option<TypeId> {
        let (kind, target) = self.dependency(node)?;
        let top_level = self.top_level_scopes[target as usize];
        let module = (kind == ImportKind::Import).then(|| {
            let map = self.model.insert_map(MapOrigin::Module(target));

            self.object_type(map)
        });

        if self.pass.visited_documents.insert(target) {
            let previous = (self.current_document, self.current_scope);

            self.current_document = target;
            if kind == ImportKind::Import {
                self.current_scope = top_level;
            }

            if let Some(module) = module {
                let slot = self.model.insert_variable(top_level, MODULE);

                self.widen(slot, module);
            }

            let body = self.ast().body();

            self.infer_block(body);
            (self.current_document, self.current_scope) = previous;
        }

        let module = module?;

        Some(self.lookup().member_type(module, KeyRef::String(EXPORTS)))
    }

    pub(crate) fn identifier_name(&self, node: NodeId) -> Option<&'a str> {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Identifier(identifier) => Some(ast.resolve(identifier.name)),
            _ => None,
        }
    }

    pub(crate) fn widen(&mut self, slot: SlotId, ty: TypeId) {
        let Some(current) = self.model.known_slot_type(slot) else {
            self.pass.has_changes |= self.model.widen(&mut self.types, slot, ty);

            return;
        };
        let union_type = self.types.union(current, ty);
        let specialized_type = self.merge_generic_instances(union_type);

        self.pass.has_changes |= self.model.replace_slot_type(slot, specialized_type);
    }

    pub(crate) fn resolve_variable(&self, name: &str) -> Option<SlotId> {
        self.model.resolve_variable(self.current_scope, name)
    }

    pub(crate) fn object_type(&mut self, map: MapId) -> TypeId {
        self.types.insert(Type::Object(map))
    }

    pub(crate) fn record_node_type(&mut self, node: NodeId, ty: TypeId) {
        if self.call_context.is_some() {
            let site = self.site(node);
            let union_type = match self.context_types.get(&site) {
                Some(&previous) => self.types.union(previous, ty),
                None => ty,
            };

            self.context_types.insert(site, union_type);

            return;
        }

        let node_type = &mut self.node_types[self.current_document as usize][node.index()];

        *node_type = Some(match *node_type {
            Some(previous) => self.types.union(previous, ty),
            None => ty,
        });
    }

    pub(crate) fn add_reference(&mut self, node: NodeId, slots: &[SlotId]) {
        self.references.add_reference(self.site(node), slots);
    }

    pub(crate) fn add_definition(&mut self, node: NodeId, slots: &[SlotId]) {
        self.references.add_definition(self.site(node), slots);
    }

    pub(crate) fn node_type(&self, node: NodeId) -> Option<TypeId> {
        if self.call_context.is_some() {
            return self.context_types.get(&self.site(node)).copied();
        }

        self.node_types[self.current_document as usize][node.index()]
    }

    pub(crate) fn report(&mut self, kind: DiagnosticKind, node: NodeId, message: String) {
        if self.call_context.is_some() {
            return;
        }

        self.report_in(self.current_document, kind, node, message);
    }

    pub(crate) fn report_in(
        &mut self,
        document: DocumentIndex,
        kind: DiagnosticKind,
        node: NodeId,
        message: String,
    ) {
        let ast = self.documents[document as usize].ast;

        self.report_span(document, kind, ast.span(node), message);
    }

    pub(crate) fn report_span(
        &mut self,
        document: DocumentIndex,
        kind: DiagnosticKind,
        span: Span,
        message: String,
    ) {
        let lines = self.documents[document as usize].ast.lines();

        self.pass
            .diagnostics
            .push((document, Diagnostic::new(kind, lines, span, message)));
    }

    pub(crate) fn infer_block(&mut self, body: &[NodeId]) {
        let mut terminator = None;

        for &statement in body {
            if matches!(self.ast().kind(statement), NodeKind::Noop(_)) {
                continue;
            }

            if let Some(terminator) = terminator.take() {
                self.report(
                    DiagnosticKind::DeadCodeAfterTerminator,
                    statement,
                    format!("Dead code after {terminator}."),
                );
            }

            self.infer_statement(statement);
            terminator = match self.ast().kind(statement) {
                NodeKind::Return(_) => Some(Keyword::Return.as_str()),
                NodeKind::Break(_) => Some(Keyword::Break.as_str()),
                NodeKind::Continue(_) => Some(Keyword::Continue.as_str()),
                _ => terminator,
            };
        }
    }
}

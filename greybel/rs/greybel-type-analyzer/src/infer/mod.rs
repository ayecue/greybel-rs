mod annotation;
mod assignment;
mod buffers;
mod call;
mod context;
mod custom;
mod declare;
mod directive;
mod expression;
mod flow;
mod function;
mod generic;
mod instance;
mod member;
mod mutation;
mod reference;
mod statement;
mod type_statement;
mod validation;

use buffers::TypeBufferPool;
use context::{Binding, BindingSet, BindingUse};
use custom::TypeDeclaration;
use flow::{
    AccessPathTable, FlowState, KindSet, LoopFrame, SideEffect, SideEffectFrame, SideEffectTable,
};
use greybel_core::{Ast, Atom, NodeId, NodeKind, Span};
use greybel_documents::ImportKind;
use rustc_hash::{FxHashMap, FxHashSet};
use validation::{ClassTable, DeferredDiagnostic};

use crate::analysis::{Analysis, ReferenceTable};
use crate::annotation::CommentTable;
use crate::api::Api;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::lookup::Lookup;
use crate::model::{
    CallContextId, DocumentIndex, FunctionId, GenericInstanceId, MapId, MapOrigin, NameId, ScopeId,
    SemanticModel, Site, SlotId,
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
    node_type_log: Vec<(DocumentIndex, NodeId, Option<TypeId>)>,
    repeatable_loops: usize,
    pass: Pass,
    current_document: DocumentIndex,
    current_scope: ScopeId,
    type_parameters_in_scope: Vec<(String, TypeId)>,
    type_aliases: FxHashMap<String, type_statement::TypeAlias>,
    /// The aliases being resolved, to stop at a cyclic alias.
    resolving_aliases: Vec<String>,
    /// The `readonly` members of custom types: the type's map and the member name.
    readonly_members: FxHashSet<(MapId, Box<str>)>,
    /// The optional members of custom types (`name?: T`, `@property {T} [name]`).
    optional_members: FxHashSet<(MapId, Box<str>)>,
    implicit_generics: bool,
    custom_map: Option<(Site, MapId)>,
    flow: FlowState,
    access_paths: AccessPathTable,
    loop_stack: Vec<LoopFrame>,
    builds_own_file: bool,
    side_effect_stack: Vec<SideEffectFrame>,
    previous: PreviousPass,
    context: ContextState,
    atom_names: Vec<Box<[NameId]>>,
    api_methods: FxHashMap<((usize, usize), TypeId), Option<TypeId>>,
    binding_use: FxHashMap<Site, BindingUse>,
    binding_changes: FxHashMap<FunctionId, Vec<(Binding, SlotId)>>,
    own_slots: FxHashMap<FunctionId, Vec<SlotId>>,
    /// Each type has one id, so a type restricted to some kinds is the same in every pass.
    restricted_types: FxHashMap<(TypeId, KindSet), TypeId>,
    /// The calls of type guards (`value is Dog` return types): the argument index and the type it is tested for.
    guard_calls: FxHashMap<(DocumentIndex, NodeId), (usize, TypeId)>,
    /// The literal type of each string and number literal node read by `literal_type_of`.
    literal_types: FxHashMap<(DocumentIndex, NodeId), Option<TypeId>>,
    /// The types of `enum` names in annotations: the union of their values' literal types.
    enum_types: FxHashMap<String, TypeId>,
    /// The names of `const enum`s, whose entries builds write as values.
    constant_enums: FxHashSet<String>,
    opaque_call_maps: FxHashSet<MapId>,
    /// The classes of `class` statements: their names, private members and methods.
    class_statements: ClassTable,
    classes: FxHashSet<MapId>,
    indirect_additions: FxHashMap<Site, BindingSet>,
    type_buffers: TypeBufferPool,
}

#[derive(Default)]
struct PreviousPass {
    loop_back_states: FxHashMap<(Site, Option<CallContextId>), FlowState>,
    function_side_effects: SideEffectTable,
}

#[derive(Default)]
struct ContextState {
    current: Option<CallContextId>,
    depth: usize,
    height: usize,
    spare_references: Vec<ReferenceTable>,
}

#[derive(Default)]
struct Pass {
    number: usize,
    has_changes: bool,
    visited_documents: FxHashSet<DocumentIndex>,
    inferred_contexts: FxHashSet<CallContextId>,
    generic_instance_set: FxHashSet<GenericInstanceId>,
    diagnostics: Vec<(DocumentIndex, Diagnostic)>,
    deferred_diagnostics: Vec<DeferredDiagnostic>,
    function_side_effects: SideEffectTable,
    document_side_effects: FxHashMap<DocumentIndex, SideEffect>,
    invalid_operations: FxHashSet<Site>,
}

impl<'a> Inference<'a> {
    pub(crate) fn run(documents: Vec<Document<'a>>, api: Api) -> Analysis {
        let mut inference = Self::new(documents, api);

        inference.insert_singleton_maps();

        inference.declare();
        for number in 0..MAX_PASSES {
            inference.pass = Pass {
                number,
                ..Pass::default()
            };
            inference.flow = FlowState::program_start();
            inference.pass.visited_documents.insert(0);
            for node_types in &mut inference.node_types {
                node_types.fill(None);
            }

            inference.define_types();

            let body = inference.documents[0].ast.body();

            inference.infer_block(body);
            inference.finish_pass_side_effects();
            if !inference.pass.has_changes {
                break;
            }
        }

        inference.finish()
    }

    fn new(documents: Vec<Document<'a>>, api: Api) -> Self {
        let mut model = SemanticModel::default();
        let atom_names = documents
            .iter()
            .map(|document| {
                document
                    .ast
                    .nodes()
                    .atoms()
                    .texts()
                    .map(|text| model.insert_name(text))
                    .collect()
            })
            .collect();

        Self {
            api,
            types: TypeTable::default(),
            model,
            references: ReferenceTable::new(
                documents.iter().map(|document| document.ast.nodes().len()),
            ),
            top_level_scopes: vec![ScopeId::Global; documents.len()],
            type_declarations: Vec::new(),
            node_types: documents
                .iter()
                .map(|document| vec![None; document.ast.nodes().len()])
                .collect(),
            node_type_log: Vec::new(),
            repeatable_loops: 0,
            documents,
            pass: Pass::default(),
            current_document: 0,
            current_scope: ScopeId::Global,
            type_parameters_in_scope: Vec::new(),
            type_aliases: FxHashMap::default(),
            resolving_aliases: Vec::new(),
            readonly_members: FxHashSet::default(),
            optional_members: FxHashSet::default(),
            implicit_generics: false,
            custom_map: None,
            flow: FlowState::default(),
            access_paths: AccessPathTable::default(),
            loop_stack: Vec::new(),
            builds_own_file: false,
            side_effect_stack: Vec::new(),
            previous: PreviousPass::default(),
            context: ContextState::default(),
            atom_names,
            api_methods: FxHashMap::default(),
            binding_use: FxHashMap::default(),
            binding_changes: FxHashMap::default(),
            own_slots: FxHashMap::default(),
            restricted_types: FxHashMap::default(),
            guard_calls: FxHashMap::default(),
            literal_types: FxHashMap::default(),
            enum_types: FxHashMap::default(),
            constant_enums: FxHashSet::default(),
            opaque_call_maps: FxHashSet::default(),
            class_statements: ClassTable::default(),
            classes: FxHashSet::default(),
            indirect_additions: FxHashMap::default(),
            type_buffers: TypeBufferPool::default(),
        }
    }

    fn finish(mut self) -> Analysis {
        self.validate_variables();
        self.validate_deprecated_uses();
        self.validate_declarations();
        self.validate_inheritance();
        self.validate_implementations();
        self.validate_implicit_any();
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

        let mut named_types = std::mem::take(&mut self.enum_types);
        let aliases: Vec<String> = self.type_aliases.keys().cloned().collect();

        for name in aliases {
            if let Some(ty) = self.alias_type(&name, &[]) {
                named_types.insert(name, ty);
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
            named_types,
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

    pub(crate) fn identifier_name_id(&self, node: NodeId) -> Option<NameId> {
        match self.ast().kind(node) {
            NodeKind::Identifier(identifier) => Some(self.atom_name(identifier.name)),
            _ => None,
        }
    }

    pub(crate) fn atom_name(&self, atom: Atom) -> NameId {
        self.atom_names[self.current_document as usize][atom.index()]
    }

    pub(crate) fn resolve_variable(&self, name: NameId) -> Option<SlotId> {
        self.model
            .visible_variables(self.current_scope, name)
            .find(|&slot| !self.flow.is_unassigned(slot))
    }

    fn insert_singleton_maps(&mut self) {
        for &function in self.api.singleton_map_functions() {
            let map = self.model.insert_map(MapOrigin::Singleton(function));
            let data = self.model.map(map);
            let (key, value) = (data.dynamic_key, data.dynamic_value);
            let ty = self.object_type(map);

            self.widen(key, TypeId::ANY);
            self.widen(value, TypeId::ANY);
            self.model.insert_singleton_type(function, ty);
        }
    }

    pub(crate) fn object_type(&mut self, map: MapId) -> TypeId {
        self.types.insert(Type::Object(map))
    }

    pub(crate) fn register_node_type(&mut self, node: NodeId, ty: TypeId) {
        if self.context.current.is_some() {
            return;
        }

        let node_type = &mut self.node_types[self.current_document as usize][node.index()];

        if self.repeatable_loops > 0 {
            self.node_type_log
                .push((self.current_document, node, *node_type));
        }

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

    pub(crate) fn report(&mut self, kind: DiagnosticKind, node: NodeId, message: String) {
        if self.context.current.is_some() {
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
}

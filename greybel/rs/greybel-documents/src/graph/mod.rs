mod builder;
mod module;

use builder::GraphBuilder;
use greybel_core::Ast;
pub use module::{Edge, Module, ModuleId};
use rustc_hash::FxHashSet;
use vscode_uri::Uri;

use crate::error::Result;
use crate::import::ImportKind;
use crate::options::DocumentOptions;
use crate::provider::{AsyncResourceProvider, ResourceProvider};
use crate::store::{Db, SourceFile};
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    pub from: Uri,
    pub to: Uri,
}

#[derive(Default)]
pub struct ModuleGraph {
    modules: Vec<Module>,
    native_imports: Vec<Vec<ModuleId>>,
    cycle: Option<Cycle>,
}

impl ModuleGraph {
    pub fn load(
        entry: &Uri,
        options: &DocumentOptions,
        provider: &impl ResourceProvider,
    ) -> Result<Self> {
        let mut workspace = Workspace::new(options.clone());

        workspace.load(entry, provider)?;

        workspace.module_graph(entry)
    }

    pub async fn load_async(
        entry: &Uri,
        options: &DocumentOptions,
        provider: &impl AsyncResourceProvider,
    ) -> Result<Self> {
        let mut workspace = Workspace::new(options.clone());

        workspace.load_async(entry, provider).await?;

        workspace.module_graph(entry)
    }

    pub(crate) fn build(db: &dyn Db, entry: SourceFile) -> Self {
        GraphBuilder::new(db).build(entry)
    }

    pub(super) fn add(&mut self, module: Module) -> ModuleId {
        let id = ModuleId(u32::try_from(self.modules.len()).expect("fewer than 2^32 modules"));

        self.modules.push(module);

        id
    }

    pub(super) fn record_cycle(&mut self, cycle: Cycle) {
        self.cycle.get_or_insert(cycle);
    }

    #[must_use]
    pub fn cycle(&self) -> Option<&Cycle> {
        self.cycle.as_ref()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn ids(&self) -> impl Iterator<Item = ModuleId> {
        (0..self.modules.len()).map(|index| ModuleId(index as u32))
    }

    pub(super) fn index_native_imports(&mut self) {
        let mut index = vec![None; self.modules.len()];

        for id in self.ids() {
            self.collect_native_imports(id, &mut index);
        }

        self.native_imports = index.into_iter().map(Option::unwrap_or_default).collect();
    }

    fn collect_native_imports(
        &self,
        id: ModuleId,
        index: &mut Vec<Option<Vec<ModuleId>>>,
    ) -> Vec<ModuleId> {
        if let Some(natives) = &index[id.index()] {
            return natives.clone();
        }

        index[id.index()] = Some(Vec::new());

        let mut unique_ids = FxHashSet::default();
        let mut natives = Vec::new();

        for edge in self.module(id).dependencies() {
            let below = self.collect_native_imports(edge.module, index);
            let own = (edge.kind == ImportKind::NativeImport).then_some(edge.module);

            natives.extend(
                below
                    .into_iter()
                    .chain(own)
                    .filter(|&id| unique_ids.insert(id)),
            );
        }

        index[id.index()] = Some(natives.clone());

        natives
    }

    #[must_use]
    pub fn entry(&self) -> Option<ModuleId> {
        (!self.modules.is_empty()).then_some(ModuleId(0))
    }

    #[must_use]
    pub fn native_imports(&self, id: ModuleId) -> &[ModuleId] {
        &self.native_imports[id.index()]
    }

    #[must_use]
    pub fn dependency_order(&self, id: ModuleId) -> Vec<&Ast> {
        let mut asts = Vec::new();

        self.push_dependency_order(id, &mut Vec::new(), &mut asts);

        asts
    }

    fn push_dependency_order<'g>(
        &'g self,
        id: ModuleId,
        stack: &mut Vec<ModuleId>,
        asts: &mut Vec<&'g Ast>,
    ) {
        if stack.contains(&id) {
            return;
        }

        let module = self.module(id);

        asts.push(&module.ast);
        stack.push(id);
        for edge in module.dependencies() {
            self.push_dependency_order(edge.module, stack, asts);
        }

        stack.pop();
    }

    #[must_use]
    pub fn module(&self, id: ModuleId) -> &Module {
        &self.modules[id.index()]
    }

    pub(super) fn module_mut(&mut self, id: ModuleId) -> &mut Module {
        &mut self.modules[id.index()]
    }
}

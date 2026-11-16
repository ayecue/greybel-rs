use std::sync::Arc;

use rustc_hash::FxHashMap;

use super::{Cycle, Edge, Module, ModuleGraph, ModuleId};
use crate::import::ImportKind;
use crate::store::{Db, SourceFile};

pub(super) struct GraphBuilder<'a> {
    db: &'a dyn Db,
    graph: ModuleGraph,
    ids: FxHashMap<SourceFile, ModuleId>,
    stack: Vec<ModuleId>,
}

impl<'a> GraphBuilder<'a> {
    pub(super) fn new(db: &'a dyn Db) -> Self {
        Self {
            db,
            graph: ModuleGraph::default(),
            ids: FxHashMap::default(),
            stack: Vec::new(),
        }
    }

    pub(super) fn build(mut self, entry: SourceFile) -> ModuleGraph {
        self.add(entry);

        let mut graph = self.graph;

        graph.index_native_imports();

        graph
    }

    fn add(&mut self, file: SourceFile) -> Option<ModuleId> {
        let db = self.db;

        file.text(db).as_ref()?;

        let ast = file.parse(db).ast()?;
        let uri = file.uri(db);
        let id = self.graph.add(Module::new(uri.clone(), Arc::clone(ast)));

        self.ids.insert(file, id);

        let links = file.links(db);
        let imports = file.imports(db);

        self.stack.push(id);
        for import in imports
            .dependencies()
            .filter(|import| import.kind.runs_code())
        {
            let Some(&target) = links.get(&import.target) else {
                continue;
            };
            let dependency = match self.ids.get(&target) {
                Some(&dependency) if self.stack.contains(&dependency) => {
                    self.graph.record_cycle(Cycle {
                        from: uri.clone(),
                        to: target.uri(db).clone(),
                    });

                    dependency
                }
                Some(&dependency) => dependency,
                None => match self.add(target) {
                    Some(dependency) => dependency,
                    None => continue,
                },
            };

            if import.kind == ImportKind::Import {
                self.graph.module_mut(dependency).register_import();
            }

            self.graph.module_mut(id).add_dependency(Edge {
                node: import.node,
                kind: import.kind,
                module: dependency,
            });
        }

        for injection in imports.injections() {
            let content = links
                .get(&injection.target)
                .and_then(|target| target.text(db).clone())
                .filter(|content| !content.is_empty());

            if let Some(content) = content {
                self.graph
                    .module_mut(id)
                    .add_injection(injection.node, content);
            }
        }

        self.stack.pop();

        Some(id)
    }
}

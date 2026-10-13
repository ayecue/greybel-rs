//! Call-recording resource provider and module-graph snapshot helpers.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io;

use greybel_documents::{AsyncResourceProvider, ModuleGraph, ModuleId, ResourceProvider, Uri};

/// Wraps `inner` and logs ordered `resolve <target>`, `relative <source> <target>`, and `read <uri>` calls with
/// URI labels.
pub struct CallRecorder<P> {
    pub inner: P,
    calls: RefCell<Vec<String>>,
}

impl<P> CallRecorder<P> {
    #[must_use]
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            calls: RefCell::default(),
        }
    }

    /// Drains recorded calls in their original order.
    pub fn take(&self) -> Vec<String> {
        self.calls.take()
    }

    /// Finds repeated pending `read` entries after each URI's first read.
    #[must_use]
    pub fn repeated_reads(&self) -> Vec<String> {
        let mut first_reads = HashSet::new();

        self.calls
            .borrow()
            .iter()
            .filter(|call| call.starts_with("read ") && !first_reads.insert(call.as_str()))
            .cloned()
            .collect()
    }

    fn record(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }
}

impl<P: ResourceProvider> ResourceProvider for CallRecorder<P> {
    fn resolve(&self, target: &str) -> Uri {
        self.record(format!("resolve {target}"));

        self.inner.resolve(target)
    }

    fn relative_to(&self, source: &Uri, target: &str) -> Uri {
        self.record(format!("relative {} {target}", source.label()));

        self.inner.relative_to(source, target)
    }

    fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        self.record(format!("read {}", uri.label()));

        self.inner.read(uri)
    }
}

impl<P: ResourceProvider> AsyncResourceProvider for CallRecorder<P> {
    async fn resolve(&self, target: &str) -> io::Result<Uri> {
        Ok(ResourceProvider::resolve(self, target))
    }

    async fn relative_to(&self, source: &Uri, target: &str) -> io::Result<Uri> {
        Ok(ResourceProvider::relative_to(self, source, target))
    }

    async fn read(&self, uri: &Uri) -> io::Result<Option<String>> {
        ResourceProvider::read(self, uri)
    }
}

/// Emits depth-first `label id [dependency ids]` lines once per module, followed by `cycle <from> -> <to>` when
/// the graph has a cycle.
#[must_use]
pub fn graph_outline(graph: &ModuleGraph) -> Vec<String> {
    fn visit(
        graph: &ModuleGraph,
        id: ModuleId,
        visits: &mut Vec<ModuleId>,
        lines: &mut Vec<String>,
    ) {
        if visits.contains(&id) {
            return;
        }

        visits.push(id);

        let module = graph.module(id);
        let dependencies: Vec<_> = module
            .dependencies()
            .iter()
            .map(|edge| edge.module)
            .collect();

        lines.push(format!("{} {id:?} {dependencies:?}", module.uri.label()));
        for dependency in dependencies {
            visit(graph, dependency, visits, lines);
        }
    }

    let mut lines = Vec::new();

    if let Some(entry) = graph.entry() {
        visit(graph, entry, &mut Vec::new(), &mut lines);
    }

    if let Some(cycle) = graph.cycle() {
        lines.push(format!(
            "cycle {} -> {}",
            cycle.from.label(),
            cycle.to.label()
        ));
    }

    lines
}

use greybel_documents::{ModuleGraph, Uri};
use greybel_source_map::{Location, SourceMap};

use crate::emit::Origin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub target: Uri,
    pub code: String,
    pub source_map: SourceMap,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Part {
    pub(super) text: String,
    pub(super) origins: Vec<Option<Origin>>,
}

impl Part {
    pub(super) fn new(text: String, origins: &[Option<Origin>]) -> Self {
        let lines = text.matches('\n').count() + 1;
        let mut origins = origins.to_vec();

        origins.resize(lines, None);

        Self { text, origins }
    }

    pub(super) fn plain(text: String) -> Self {
        Self::new(text, &[])
    }
}

#[derive(Default)]
pub(super) struct PendingOutput {
    text: String,
    origins: Vec<Option<Origin>>,
    started: bool,
}

impl PendingOutput {
    pub(super) fn push(&mut self, part: &Part) {
        if self.started {
            self.text.push('\n');
        }

        self.started = true;
        self.text.push_str(&part.text);
        self.origins.extend_from_slice(&part.origins);
    }

    pub(super) fn finish(self, target: Uri, graph: &ModuleGraph) -> Output {
        let uris: Vec<String> = graph
            .ids()
            .map(|id| graph.module(id).uri.to_string())
            .collect();
        let source_map = SourceMap::of_lines(self.origins.iter().map(|origin| {
            origin.map(|origin| Location::new(&uris[origin.module.index()], origin.line))
        }));

        Output {
            target,
            code: self.text,
            source_map,
        }
    }
}

use greybel_core::NodeId;
use greybel_core::syntax::greyscript::ImportCode;
use greybel_symbols::contextual_keywords::{NO_EMIT, NO_EVAL, OVERRIDE};
use greybel_symbols::delimiters::{LINE_COMMENT, STATEMENT_SEPARATOR};

use super::EmitState;

impl EmitState<'_> {
    pub(super) fn push_import_code_source(&mut self, node: &ImportCode, with_flags: bool) {
        let original = self.read(node.original_directory);

        self.push_fmt(format_args!("import_code(\"{original}\")"));
        if !with_flags {
            return;
        }

        let mut flags = Vec::new();

        if !node.eval {
            flags.push(NO_EVAL.to_owned());
        }

        if !node.emit {
            flags.push(NO_EMIT.to_owned());
        }

        let directory = self.read(node.directory);

        if directory != original {
            flags.push(format!("{OVERRIDE}={directory}"));
        }

        if !flags.is_empty() {
            self.push_fmt(format_args!(
                "{LINE_COMMENT}{}{STATEMENT_SEPARATOR}",
                flags.join(STATEMENT_SEPARATOR)
            ));
        }
    }

    pub(super) fn bundled_imports(&mut self, id: NodeId) -> Vec<String> {
        let Some(graph) = self.graph() else {
            return Vec::new();
        };

        if self.output_module() != graph.entry() {
            return Vec::new();
        }

        let Some(dependency) = self.module().and_then(|module| module.dependency(id)) else {
            return Vec::new();
        };
        let options = self.options();
        let path = |id| {
            let uri = &graph.module(id).uri;

            options
                .path_mapping
                .as_ref()
                .map_or_else(|| uri.label(), |mapping| mapping.map(uri))
        };
        let native_imports = self.emitted_native_imports_mut();

        if native_imports.contains(&dependency) {
            return Vec::new();
        }

        let dependency_native_imports = graph.native_imports(dependency).iter().copied();
        let lines = dependency_native_imports
            .filter(|&id| native_imports.insert(id))
            .map(path)
            .collect::<Vec<_>>();

        native_imports.insert(dependency);

        lines.into_iter().chain([path(dependency)]).collect()
    }
}

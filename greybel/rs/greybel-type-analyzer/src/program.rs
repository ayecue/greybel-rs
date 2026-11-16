use std::collections::VecDeque;
use std::collections::hash_map::Entry;

use greybel_core::{Dialect, Keyword, Span};
use greybel_documents::SourceFile;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::analysis::Analysis;
use crate::api::Api;
use crate::database::{Annotate, Db};
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::infer::{Document, Inference};
use crate::model::DocumentIndex;

pub(crate) struct Program {
    documents: Vec<SourceFile>,
    index: FxHashMap<SourceFile, DocumentIndex>,
}

impl Program {
    pub(crate) fn new(db: &dyn Db, file: SourceFile) -> Self {
        let mut program = Self {
            documents: vec![file],
            index: FxHashMap::from_iter([(file, 0)]),
        };
        let mut next = 0;

        while let Some(&document) = program.documents.get(next) {
            next += 1;
            for &(kind, target) in document.resolved_imports(db) {
                let exists = target.text(db).is_some() && target.parse(db).ast().is_some();

                if kind.is_dependency() && exists && !program.index.contains_key(&target) {
                    let index = DocumentIndex::try_from(program.documents.len())
                        .expect("fewer than 2^32 documents");

                    program.index.insert(target, index);
                    program.documents.push(target);
                }
            }
        }

        program
    }

    pub(crate) fn analyze(self, db: &dyn Db) -> Option<Analysis> {
        let api = Api::new(self.documents[0].project(db).options(db).parser.dialect);
        let documents = self
            .documents
            .iter()
            .map(|&document| {
                let links = document.links(db);
                let dependencies = document
                    .imports(db)
                    .dependencies()
                    .filter_map(|import| {
                        let target = self.index.get(links.get(&import.target)?)?;

                        Some((import.node, (import.kind, *target)))
                    })
                    .collect();

                Some(Document {
                    ast: document.parse(db).ast()?,
                    comments: document.comments(db),
                    dependencies,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let mut analysis = Inference::run(documents, api);

        for (index, &document) in self.documents.iter().enumerate() {
            let index = DocumentIndex::try_from(index).expect("value fits the target integer type");

            analysis.add_diagnostics(index, Self::missing_import_diagnostics(db, document));
            analysis.add_diagnostics(index, self.cyclic_import_diagnostics(db, document));
            analysis.add_diagnostics(index, Self::ignored_import_code_diagnostics(db, document));
        }

        analysis.set_documents(self.documents);

        Some(analysis)
    }

    fn cyclic_import_diagnostics(&self, db: &dyn Db, document: SourceFile) -> Vec<Diagnostic> {
        let Some(ast) = document.parse(db).ast() else {
            return Vec::new();
        };
        let links = document.links(db);

        document
            .imports(db)
            .dependencies()
            .filter_map(|import| {
                let target = *links.get(&import.target)?;
                let path = self.path_between(db, target, document)?;
                let cycle: Vec<String> = std::iter::once(document)
                    .chain(path)
                    .map(|file| file.uri(db).label())
                    .collect();

                Some(Diagnostic::new(
                    DiagnosticKind::CyclicImport,
                    ast.lines(),
                    ast.span(import.node),
                    format!("Cyclic import: {}.", cycle.join(" -> ")),
                ))
            })
            .collect()
    }

    fn path_between(
        &self,
        db: &dyn Db,
        from: SourceFile,
        to: SourceFile,
    ) -> Option<Vec<SourceFile>> {
        if !self.index.contains_key(&from) {
            return None;
        }

        let mut previous = FxHashMap::from_iter([(from, from)]);
        let mut file_queue = VecDeque::from([from]);

        while let Some(file) = file_queue.pop_front() {
            if file == to {
                let mut path = vec![to];
                let mut current = to;

                while current != from {
                    current = previous[&current];
                    path.push(current);
                }

                path.reverse();

                return Some(path);
            }

            for dependency in self.dependencies(db, file) {
                if let Entry::Vacant(entry) = previous.entry(dependency) {
                    entry.insert(file);
                    file_queue.push_back(dependency);
                }
            }
        }

        None
    }

    fn dependencies(&self, db: &dyn Db, file: SourceFile) -> Vec<SourceFile> {
        let links = file.links(db);

        file.imports(db)
            .dependencies()
            .filter_map(|import| links.get(&import.target).copied())
            .filter(|target| self.index.contains_key(target))
            .collect()
    }

    fn missing_import_diagnostics(db: &dyn Db, document: SourceFile) -> Vec<Diagnostic> {
        let Some(ast) = document.parse(db).ast() else {
            return Vec::new();
        };
        let links = document.links(db);

        document
            .imports(db)
            .iter()
            .filter_map(|import| {
                let target = links.get(&import.target)?;

                target.text(db).is_none().then(|| {
                    Diagnostic::new(
                        DiagnosticKind::MissingImport,
                        ast.lines(),
                        ast.span(import.node),
                        format!("Cannot find the imported file \"{}\".", import.target),
                    )
                })
            })
            .collect()
    }

    fn ignored_import_code_diagnostics(db: &dyn Db, document: SourceFile) -> Vec<Diagnostic> {
        if document.project(db).options(db).parser.dialect == Dialect::MiniScript {
            return Vec::new();
        }

        let Some(ast) = document.parse(db).ast() else {
            return Vec::new();
        };
        let call = format!("{}(", Keyword::ImportCode.as_str());
        let source = ast.source();
        let statements: FxHashSet<u32> = ast
            .native_imports
            .iter()
            .map(|&node| ast.span(node).start)
            .collect();

        source
            .match_indices(&call)
            .filter_map(|(start, _)| {
                let start = u32::try_from(start).ok()?;
                let end = start + u32::try_from(call.len()).ok()?;
                let is_skipped = source[..start as usize].ends_with("//");

                (!is_skipped && !statements.contains(&start)).then(|| {
                    Diagnostic::new(
                        DiagnosticKind::IgnoredImportCode,
                        ast.lines(),
                        Span::new(start, end),
                        "Grey Hack imports this import_code even in a comment or a string; only one written directly after // is skipped.".to_owned(),
                    )
                })
            })
            .collect()
    }
}

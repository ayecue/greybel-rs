use greybel_documents::{SourceFile, Uri};
use rustc_hash::FxHashMap;

use super::Analyzer;
use crate::analysis::Analysis;
use crate::diagnostic::Diagnostic;
use crate::model::DocumentIndex;

impl<'w> Analyzer<'w> {
    #[must_use]
    pub fn entry_uris(&self, uri: &Uri) -> Vec<Uri> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };
        let db = self.workspace.db();

        self.entry_files(file)
            .into_iter()
            .map(|entry| entry.uri(db).clone())
            .collect()
    }

    pub(super) fn entry_files(&self, file: SourceFile) -> Vec<SourceFile> {
        let db = self.workspace.db();
        let mut importers: FxHashMap<SourceFile, Vec<SourceFile>> = FxHashMap::default();

        for origin in self.workspace.files() {
            if origin.text(db).is_none() {
                continue;
            }

            for &(kind, target) in origin.resolved_imports(db) {
                if kind.is_dependency() && target != origin {
                    importers.entry(target).or_default().push(origin);
                }
            }
        }

        let mut file_queue = vec![file];
        let mut next = 0;

        while let Some(&current) = file_queue.get(next) {
            next += 1;
            for &runner in importers.get(&current).into_iter().flatten() {
                if !file_queue.contains(&runner) {
                    file_queue.push(runner);
                }
            }
        }

        let mut entries: Vec<SourceFile> = file_queue
            .into_iter()
            .filter(|candidate| !importers.contains_key(candidate))
            .collect();

        if entries.is_empty() {
            entries.push(file);
        }

        entries.sort_by(|a, b| a.uri(db).cmp(b.uri(db)));

        entries
    }

    pub(super) fn entry_analyses(
        &self,
        file: SourceFile,
    ) -> Vec<(SourceFile, &'w Analysis, DocumentIndex)> {
        let analyses: Vec<(SourceFile, &'w Analysis, DocumentIndex)> = self
            .entry_files(file)
            .into_iter()
            .filter_map(|entry| {
                let analysis = self.file_analysis(entry)?;

                Some((entry, analysis, analysis.document(file)?))
            })
            .collect();

        if analyses.is_empty() {
            return self
                .file_analysis(file)
                .map(|analysis| (file, analysis, 0))
                .into_iter()
                .collect();
        }

        analyses
    }

    pub(super) fn entry_diagnostics(&self, file: SourceFile) -> Vec<Diagnostic> {
        let entries = self.entry_analyses(file);
        let mut grouped_diagnostics: Vec<(Diagnostic, Vec<SourceFile>)> = Vec::new();

        for &(entry, analysis, document) in &entries {
            for diagnostic in analysis.diagnostics(document) {
                match grouped_diagnostics
                    .iter_mut()
                    .find(|(it, _)| it == diagnostic)
                {
                    Some((_, reporters)) => reporters.push(entry),
                    None => grouped_diagnostics.push((diagnostic.clone(), vec![entry])),
                }
            }
        }

        let db = self.workspace.db();
        let mut diagnostics: Vec<Diagnostic> = grouped_diagnostics
            .into_iter()
            .map(|(mut diagnostic, reporters)| {
                if reporters.len() < entries.len() {
                    let labels: Vec<String> = reporters
                        .iter()
                        .map(|entry| entry.uri(db).label())
                        .collect();

                    diagnostic.message = format!(
                        "{} (when run from {})",
                        diagnostic.message,
                        labels.join(", ")
                    );
                }

                diagnostic
            })
            .collect();

        Diagnostic::sort(&mut diagnostics);

        diagnostics
    }
}

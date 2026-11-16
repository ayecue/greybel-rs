mod entries;

use greybel_core::{Ast, ColumnSettings, Dialect, ParserOptions, Position, Range, Span};
use greybel_documents::{DocumentOptions, SourceFile, Uri, Workspace};

use crate::analysis::Analysis;
use crate::database::{Analyze, Annotate, Database};
use crate::diagnostic::Diagnostic;
use crate::ide::{Context, Location};
use crate::model::DocumentIndex;

#[derive(Clone, Copy)]
pub struct Analyzer<'w> {
    workspace: &'w Workspace<Database>,
}

impl<'w> Analyzer<'w> {
    #[must_use]
    pub fn new(workspace: &'w Workspace<Database>) -> Self {
        Self { workspace }
    }

    #[must_use]
    pub fn document_options(dialect: Dialect) -> DocumentOptions {
        DocumentOptions {
            parser: ParserOptions::new(dialect),
            unsafe_mode: true,
            ..DocumentOptions::default()
        }
    }

    #[must_use]
    pub fn workspace(&self) -> &'w Workspace<Database> {
        self.workspace
    }

    #[must_use]
    pub fn analysis(&self, uri: &Uri) -> Option<&'w Analysis> {
        self.file_analysis(self.workspace.file(uri)?)
    }

    /// The diagnostics that are reported by default; [`Self::all_diagnostics`] also has those that are off by
    /// default, for [`DiagnosticOptions::apply`](crate::DiagnosticOptions::apply).
    #[must_use]
    pub fn diagnostics(&self, uri: &Uri) -> Vec<Diagnostic> {
        self.all_diagnostics(uri)
            .into_iter()
            .filter(|diagnostic| diagnostic.kind.is_enabled_by_default())
            .collect()
    }

    #[must_use]
    pub fn all_diagnostics(&self, uri: &Uri) -> Vec<Diagnostic> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };
        let parse = file.parse(self.workspace.db());
        let Some(ast) = parse.ast() else {
            return Vec::new();
        };

        let ignores = &file.comments(self.workspace.db()).ignores;

        parse
            .errors()
            .iter()
            .map(|error| Diagnostic::syntax(error, ast.lines()))
            .chain(self.entry_diagnostics(file))
            .filter(|diagnostic| !ignores.ignores(diagnostic))
            .chain(ignores.invalid.iter().cloned())
            .collect()
    }

    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.workspace.options().parser.dialect
    }

    pub(crate) fn file_analysis(self, file: SourceFile) -> Option<&'w Analysis> {
        let db = self.workspace.db();

        file.text(db).as_ref()?;

        file.analysis(db).as_ref()
    }

    pub(crate) fn locate(
        self,
        uri: &Uri,
        position: Position,
    ) -> Option<(SourceFile, &'w Ast, u32)> {
        let file = self.workspace.file(uri)?;
        let ast = file.parse(self.workspace.db()).ast()?;
        let offset = ast.lines().offset(position, ColumnSettings::default())?;

        Some((file, ast, offset))
    }

    pub(crate) fn ide_context(self, uri: &Uri, position: Position) -> Option<Context<'w>> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (_, analysis, document) = self.entry_analyses(file).into_iter().next()?;

        Some(Context::new(ast, analysis, document, offset))
    }

    pub(crate) fn analyses_of(self, file: SourceFile) -> Vec<(&'w Analysis, DocumentIndex)> {
        std::iter::once(file)
            .chain(self.dependent_closure(file))
            .filter_map(|candidate_root| {
                let analysis = self.file_analysis(candidate_root)?;
                let document = analysis
                    .documents()
                    .iter()
                    .position(|&candidate| candidate == file)?;

                Some((analysis, DocumentIndex::try_from(document).ok()?))
            })
            .collect()
    }

    pub(crate) fn dependent_closure(self, file: SourceFile) -> Vec<SourceFile> {
        let mut files = vec![file];
        let mut next = 0;

        while let Some(&current) = files.get(next) {
            next += 1;

            let uri = current.uri(self.workspace.db());

            for dependent in self.workspace.dependents(uri) {
                let Some(dependent) = self.workspace.file(&dependent) else {
                    continue;
                };

                if !files.contains(&dependent) {
                    files.push(dependent);
                }
            }
        }

        files.remove(0);

        files
    }

    pub(crate) fn text_in(ast: &Ast, range: Range) -> Option<&str> {
        let lines = ast.lines();
        let start = lines.offset(range.start, ColumnSettings::default())? as usize;
        let end = lines.offset(range.end, ColumnSettings::default())? as usize;

        ast.source().get(start..end)
    }

    pub(crate) fn range(ast: &Ast, span: Span) -> Range {
        ast.lines().range(span, ColumnSettings::default())
    }

    pub(crate) fn sort_locations(mut locations: Vec<Location>) -> Vec<Location> {
        locations.sort_by(|a, b| (&a.uri, a.range.start).cmp(&(&b.uri, b.range.start)));
        locations.dedup();

        locations
    }
}

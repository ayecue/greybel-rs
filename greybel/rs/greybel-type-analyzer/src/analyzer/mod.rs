mod entries;
mod imports;
mod navigation;
mod symbols;

use greybel_core::{Ast, ColumnSettings, Dialect, ParserOptions, Position};
use greybel_documents::{DocumentOptions, SourceFile, Uri, Workspace};

use crate::analysis::Analysis;
use crate::database::{Analyze, Database};
use crate::diagnostic::Diagnostic;
use crate::ide::{CompletionItem, Context, DocumentSymbol, Hover, SignatureHelp};

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

    #[must_use]
    pub fn diagnostics(&self, uri: &Uri) -> Vec<Diagnostic> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };
        let parse = file.parse(self.workspace.db());
        let Some(ast) = parse.ast() else {
            return Vec::new();
        };

        parse
            .errors()
            .iter()
            .map(|error| Diagnostic::syntax(error, ast.lines()))
            .chain(self.entry_diagnostics(file))
            .collect()
    }

    #[must_use]
    pub fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        self.ide_context(uri, position)?.hover()
    }

    #[must_use]
    pub fn completions(&self, uri: &Uri, position: Position) -> Vec<CompletionItem> {
        if let Some(paths) = self.path_completions(uri, position) {
            return paths;
        }

        self.ide_context(uri, position)
            .map(|context| context.completions())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn signature_help(&self, uri: &Uri, position: Position) -> Option<SignatureHelp> {
        self.ide_context(uri, position)?.signature_help()
    }

    #[must_use]
    pub fn document_symbols(&self, uri: &Uri) -> Vec<DocumentSymbol> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };

        file.parse(self.workspace.db())
            .ast()
            .map(|ast| DocumentSymbol::outline(ast))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.workspace.options().parser.dialect
    }

    pub(super) fn file_analysis(self, file: SourceFile) -> Option<&'w Analysis> {
        let db = self.workspace.db();

        file.text(db).as_ref()?;

        file.analysis(db).as_ref()
    }

    pub(super) fn locate(
        self,
        uri: &Uri,
        position: Position,
    ) -> Option<(SourceFile, &'w Ast, u32)> {
        let file = self.workspace.file(uri)?;
        let ast = file.parse(self.workspace.db()).ast()?;
        let offset = ast.lines().offset(position, ColumnSettings::default())?;

        Some((file, ast, offset))
    }

    fn ide_context(self, uri: &Uri, position: Position) -> Option<Context<'w>> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (_, analysis, document) = self.entry_analyses(file).into_iter().next()?;

        Some(Context::new(ast, analysis, document, offset))
    }
}

use greybel_core::{Ast, ColumnSettings, Dialect, Range, Span};
use greybel_documents::{SourceFile, Uri};

use super::{CodeAction, FixContext};
use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{DocumentSymbol, RelativePath, SymbolKind, TextEdit, WorkspaceEdit};

struct Insertion {
    range: Range,
    leading_newline: bool,
}

impl Insertion {
    fn after_directives(ast: &Ast, directives: impl Iterator<Item = Span>) -> Self {
        let source = ast.source();
        let end = directives.map(|span| span.end as usize).max();
        let (offset, leading_newline) = match end {
            None => (0, false),
            Some(end) => match source[end..].find('\n') {
                Some(newline) => (end + newline + 1, false),
                None => (source.len(), true),
            },
        };
        let offset = u32::try_from(offset).expect("value fits the target integer type");

        Self {
            range: ast
                .lines()
                .range(Span::empty(offset), ColumnSettings::default()),
            leading_newline,
        }
    }

    fn import_edit(&self, uri: &Uri, directive: &str) -> WorkspaceEdit {
        let new_text = if self.leading_newline {
            format!("\n{directive}")
        } else {
            format!("{directive}\n")
        };

        WorkspaceEdit::of_file(
            uri,
            vec![TextEdit {
                range: self.range,
                new_text,
            }],
        )
    }
}

pub(super) struct AddInclude;

impl AddInclude {
    /// `#include` (and `import_code` in GreyScript) of workspace files that define the unresolved names.
    pub(super) fn actions(context: &FixContext<'_>, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
        let unresolved_diagnostics: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == DiagnosticKind::UnresolvedReference)
            .collect();
        let FixContext {
            analyzer,
            uri,
            file,
            ast,
        } = *context;
        let dialect = analyzer.dialect();
        let db = analyzer.workspace().db();

        if dialect == Dialect::MiniScript || unresolved_diagnostics.is_empty() {
            return Vec::new();
        }

        let candidates = Self::candidates(analyzer, file);
        let insertion = Insertion::after_directives(
            ast,
            file.imports(db)
                .dependencies()
                .map(|import| ast.span(import.node)),
        );
        let mut actions = Vec::new();

        for diagnostic in unresolved_diagnostics {
            let Some(name) = Analyzer::text_in(ast, diagnostic.range) else {
                continue;
            };

            for &(candidate, ref symbols) in &candidates {
                let defines = symbols.iter().any(|symbol| {
                    symbol.name == name
                        && matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Function)
                });

                if !defines {
                    continue;
                }

                let Some(relative) = RelativePath::between(uri, candidate.uri(db)) else {
                    continue;
                };
                let mut directives = vec![(format!("#include \"{relative}\";"), true)];

                if dialect == Dialect::GreyScript {
                    directives.push((format!("import_code(\"{relative}\")"), false));
                }

                for (directive, is_preferred) in directives {
                    actions.push(CodeAction {
                        title: format!("Add {directive}"),
                        diagnostic: diagnostic.clone(),
                        edit: insertion.import_edit(uri, &directive),
                        is_preferred,
                    });
                }
            }
        }

        actions
    }

    fn candidates(
        analyzer: Analyzer<'_>,
        file: SourceFile,
    ) -> Vec<(SourceFile, Vec<DocumentSymbol>)> {
        let workspace = analyzer.workspace();
        let db = workspace.db();
        let documents: Vec<SourceFile> = analyzer
            .file_analysis(file)
            .map(|analysis| analysis.documents().to_vec())
            .unwrap_or_default();
        let dependents = analyzer.dependent_closure(file);
        let mut candidates: Vec<(SourceFile, Vec<DocumentSymbol>)> = workspace
            .files()
            .filter(|other| !documents.contains(other) && !dependents.contains(other))
            .filter_map(|other| {
                let ast = other.parse(db).ast()?;

                Some((other, DocumentSymbol::outline(ast)))
            })
            .collect();

        candidates.sort_by(|a, b| a.0.uri(db).cmp(b.0.uri(db)));

        candidates
    }
}

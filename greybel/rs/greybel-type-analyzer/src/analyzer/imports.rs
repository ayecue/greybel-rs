use greybel_core::{Ast, ColumnSettings, Dialect, Position, Range, Span};
use greybel_documents::{SourceFile, Uri};

use super::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{
    CodeAction, CompletionItem, CompletionItemKind, DocumentSymbol, ImportTarget, RelativePath,
    SymbolKind, TextEdit, WorkspaceEdit,
};

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
        let mut edit = WorkspaceEdit::default();

        edit.changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: self.range,
                new_text,
            }],
        );

        edit
    }
}

impl<'w> Analyzer<'w> {
    pub(super) fn path_completions(
        self,
        uri: &Uri,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let target = ImportTarget::at(ast.source(), offset as usize, self.dialect())?;
        let db = self.workspace.db();
        let range = ast.lines().range(
            Span::new(
                u32::try_from(target.start).expect("value fits the target integer type"),
                offset,
            ),
            ColumnSettings::default(),
        );
        let mut items: Vec<CompletionItem> = self
            .workspace
            .files()
            .filter(|&other| other != file && other.text(db).is_some())
            .filter_map(|other| {
                let other = other.uri(db);
                let relative = RelativePath::between(uri, other)?;
                let label = if target.prefix.starts_with('/') {
                    other.path().to_owned()
                } else {
                    relative
                };

                label.starts_with(target.prefix).then(|| CompletionItem {
                    text_edit: Some(TextEdit {
                        range,
                        new_text: label.clone(),
                    }),
                    label,
                    kind: CompletionItemKind::File,
                    detail: other.label(),
                    documentation: None,
                })
            })
            .collect();

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.dedup_by(|a, b| a.label == b.label);

        Some(items)
    }

    #[must_use]
    pub fn code_actions(&self, uri: &Uri, range: Range) -> Vec<CodeAction> {
        let dialect = self.dialect();
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };
        let db = self.workspace.db();
        let Some(ast) = file.parse(db).ast() else {
            return Vec::new();
        };

        if dialect == Dialect::MiniScript {
            return Vec::new();
        }

        let unresolved_diagnostics: Vec<Diagnostic> = self
            .diagnostics(uri)
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.kind == DiagnosticKind::UnresolvedReference
                    && diagnostic.range.start <= range.end
                    && range.start <= diagnostic.range.end
            })
            .collect();

        if unresolved_diagnostics.is_empty() {
            return Vec::new();
        }

        let candidates = self.include_candidates(file);
        let insertion = Insertion::after_directives(
            ast,
            file.imports(db)
                .dependencies()
                .map(|import| ast.span(import.node)),
        );
        let mut actions = Vec::new();

        for diagnostic in unresolved_diagnostics {
            let Some(name) = Self::text_in(ast, diagnostic.range) else {
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

    fn include_candidates(self, file: SourceFile) -> Vec<(SourceFile, Vec<DocumentSymbol>)> {
        let db = self.workspace.db();
        let documents: Vec<SourceFile> = self
            .file_analysis(file)
            .map(|analysis| analysis.documents().to_vec())
            .unwrap_or_default();
        let dependents = self.dependent_closure(file);
        let mut candidates: Vec<(SourceFile, Vec<DocumentSymbol>)> = self
            .workspace
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

    fn text_in(ast: &Ast, range: Range) -> Option<&str> {
        let lines = ast.lines();
        let start = lines.offset(range.start, ColumnSettings::default())? as usize;
        let end = lines.offset(range.end, ColumnSettings::default())? as usize;

        ast.source().get(start..end)
    }
}

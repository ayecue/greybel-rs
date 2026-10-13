use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Range};
use greybel_documents::Uri;

use super::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Variable,
    Function,
    Property,
}

impl DocumentSymbol {
    pub(crate) fn outline(ast: &Ast) -> Vec<Self> {
        let mut symbols = Vec::new();

        Self::collect(ast, ast.body(), &mut symbols);

        symbols
    }

    fn collect(ast: &Ast, body: &[NodeId], symbols: &mut Vec<Self>) {
        for &statement in body {
            match ast.kind(statement) {
                NodeKind::Assignment(assignment) => {
                    let Some(name) = ast.text(assignment.variable) else {
                        continue;
                    };
                    let mut children = Vec::new();
                    let kind = if let NodeKind::Function(function) = ast.kind(assignment.init) {
                        Self::collect(ast, ast.list(function.body), &mut children);

                        SymbolKind::Function
                    } else if matches!(ast.kind(assignment.variable), NodeKind::Identifier(_)) {
                        SymbolKind::Variable
                    } else {
                        SymbolKind::Property
                    };
                    let range = |node| ast.lines().range(ast.span(node), ColumnSettings::default());

                    symbols.push(Self {
                        name: name.to_owned(),
                        kind,
                        range: range(statement),
                        selection_range: range(assignment.variable),
                        children,
                    });
                }
                NodeKind::If(statement) => {
                    for &clause in ast.list(statement.clauses) {
                        match ast.kind(clause) {
                            NodeKind::IfClause(clause) => {
                                Self::collect(ast, ast.list(clause.body), symbols)
                            }
                            NodeKind::ElseClause(clause) => {
                                Self::collect(ast, ast.list(clause.body), symbols);
                            }
                            _ => {}
                        }
                    }
                }
                NodeKind::While(statement) => Self::collect(ast, ast.list(statement.body), symbols),
                NodeKind::For(statement) => Self::collect(ast, ast.list(statement.body), symbols),
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<String>,
}

impl WorkspaceSymbol {
    pub(crate) fn collect(
        uri: &Uri,
        symbols: Vec<DocumentSymbol>,
        container: Option<&str>,
        query: &SymbolQuery,
        results: &mut Vec<Self>,
    ) {
        for symbol in symbols {
            if query.matches(&symbol.name) {
                results.push(Self {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    location: Location {
                        uri: uri.clone(),
                        range: symbol.selection_range,
                    },
                    container: container.map(str::to_owned),
                });
            }

            Self::collect(uri, symbol.children, Some(&symbol.name), query, results);
        }
    }
}

pub(crate) struct SymbolQuery {
    characters: Vec<char>,
}

impl SymbolQuery {
    pub(crate) fn new(query: &str) -> Self {
        Self {
            characters: query.chars().flat_map(char::to_lowercase).collect(),
        }
    }

    pub(crate) fn matches(&self, name: &str) -> bool {
        let mut query_chars = self.characters.iter().peekable();

        for c in name.chars().flat_map(char::to_lowercase) {
            if query_chars.peek() == Some(&&c) {
                query_chars.next();
            }
        }

        query_chars.peek().is_none()
    }
}

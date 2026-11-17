use greybel_core::syntax::greybel::Class;
use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Range, Span};

use crate::annotation::CommentTable;

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
    File,
    Type,
    Class,
}

impl DocumentSymbol {
    pub(crate) fn from_ast(ast: &Ast) -> Vec<Self> {
        let mut symbols = Vec::new();

        Self::collect(ast, ast.body(), &mut symbols);

        symbols
    }

    pub(super) fn from_ast_with_types(ast: &Ast, comments: &CommentTable) -> Vec<Self> {
        let mut symbols = Self::nest_member_paths(Self::from_ast(ast));

        Self::declared_types(ast, comments, &mut symbols);
        symbols.sort_by_key(|symbol| symbol.range.start);

        symbols
    }

    fn nest_member_paths(symbols: Vec<Self>) -> Vec<Self> {
        let mut roots: Vec<Self> = Vec::new();

        for mut symbol in symbols {
            symbol.children = Self::nest_member_paths(std::mem::take(&mut symbol.children));

            if let Some((owner, member)) = symbol.name.rsplit_once('.')
                && !owner.is_empty()
                && !member.is_empty()
                && !symbol.name.contains(['[', '(', ' '])
                && let Some(parent) = Self::find_path(&mut roots, owner)
            {
                let member_start = symbol.name.len() - member.len();

                symbol.name.drain(..member_start);
                parent.range.end = parent.range.end.max(symbol.range.end);
                parent.children.push(symbol);

                continue;
            }

            roots.push(symbol);
        }

        roots
    }

    fn find_path<'a>(symbols: &'a mut [Self], path: &str) -> Option<&'a mut Self> {
        if let Some(index) = symbols.iter().rposition(|symbol| symbol.name == path) {
            return Some(&mut symbols[index]);
        }

        let (index, remainder) = path.match_indices('.').find_map(|(dot, _)| {
            let index = symbols
                .iter()
                .rposition(|symbol| symbol.name == path[..dot])?;

            Some((index, &path[dot + 1..]))
        })?;

        Self::find_path(&mut symbols[index].children, remainder)
    }

    fn tag_name_offset(text: &str, name: &str) -> Option<usize> {
        let from = match text.rfind('}') {
            Some(close) => close + 1,
            None => {
                let tag = text.find('@')?;

                tag + text[tag..].find(char::is_whitespace)?
            }
        };

        Some(from + text[from..].find(name)?)
    }

    fn declared_types(ast: &Ast, comments: &CommentTable, symbols: &mut Vec<Self>) {
        let range = |span| ast.lines().range(span, ColumnSettings::default());

        for block in &comments.blocks {
            let line = |index: usize, name: &str| {
                let span = block.spans[index];
                let text = block.text_spans[index];
                let selection = Self::tag_name_offset(ast.slice(text), name)
                    .and_then(|start| u32::try_from(start).ok())
                    .map_or(span, |start| {
                        let start = text.start + start;

                        Span::new(start, start + u32::try_from(name.len()).unwrap_or(0))
                    });

                (range(span), range(selection))
            };

            for ty in &block.doc.types {
                let (type_range, type_selection) = line(ty.line, &ty.name);
                let properties = ty.properties.iter().map(|property| {
                    let name = property.path.join(".");
                    let (range, selection_range) = line(property.line, &name);

                    Self {
                        name,
                        kind: SymbolKind::Property,
                        range,
                        selection_range,
                        children: Vec::new(),
                    }
                });
                let functions = ty.functions.iter().map(|function| {
                    let (range, selection_range) = line(function.line, &function.name);

                    Self {
                        name: function.name.clone(),
                        kind: SymbolKind::Function,
                        range,
                        selection_range,
                        children: Vec::new(),
                    }
                });

                symbols.push(Self {
                    name: ty.name.clone(),
                    kind: SymbolKind::Type,
                    range: type_range,
                    selection_range: type_selection,
                    children: properties.chain(functions).collect(),
                });
            }
        }
    }

    fn class(ast: &Ast, statement: NodeId, class: &Class) -> Option<Self> {
        let name = ast.text(class.name)?;
        let range = |node| ast.lines().range(ast.span(node), ColumnSettings::default());
        let mut children = Vec::new();

        for member in &class.members {
            let NodeKind::Assignment(assignment) = ast.kind(member.assignment) else {
                continue;
            };
            let NodeKind::Member(target) = ast.kind(assignment.variable) else {
                continue;
            };
            let Some(member_name) = ast.text(target.identifier) else {
                continue;
            };
            let mut grandchildren = Vec::new();
            let kind = if let NodeKind::Function(function) = ast.kind(assignment.init) {
                let body = class
                    .constructor
                    .as_ref()
                    .filter(|constructor| constructor.function == assignment.init)
                    .map_or(function.body, |constructor| constructor.body);

                Self::collect(ast, ast.list(body), &mut grandchildren);

                SymbolKind::Function
            } else {
                SymbolKind::Property
            };

            children.push(Self {
                name: member_name.to_owned(),
                kind,
                range: range(member.assignment),
                selection_range: range(target.identifier),
                children: grandchildren,
            });
        }

        Some(Self {
            name: name.to_owned(),
            kind: SymbolKind::Class,
            range: range(statement),
            selection_range: range(class.name),
            children,
        })
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
                NodeKind::Class(class) => symbols.extend(Self::class(ast, statement, class)),
                _ => {}
            }
        }
    }
}

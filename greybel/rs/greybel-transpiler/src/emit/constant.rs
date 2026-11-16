use greybel_core::syntax::IdentifierKind;
use greybel_core::{Ast, LiteralValue, NodeId, NodeKind};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{LiteralStyle, LiteralText};

/// The entries of the `const enum`s of the files a build writes, by enum name and entry name, as the literal text
/// builds write in place of `Name.Entry` reads. A name that two files declare with other entries is skipped.
#[derive(Debug, Default)]
pub struct ConstantTable {
    entries: FxHashMap<(String, String), String>,
    names: FxHashMap<String, NodeOwner>,
    conflicts: FxHashSet<String>,
}

/// The file and enum statement that declares a `const enum` name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NodeOwner {
    file: usize,
    node: NodeId,
}

impl ConstantTable {
    #[must_use]
    pub fn of<'a>(asts: impl IntoIterator<Item = &'a Ast>) -> Self {
        let mut table = Self::default();

        for (file, ast) in asts.into_iter().enumerate() {
            table.add(file, ast);
        }

        table
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The literal text of `name.entry`.
    #[must_use]
    pub fn get(&self, name: &str, entry: &str) -> Option<&str> {
        if self.conflicts.contains(name) {
            return None;
        }

        self.entries
            .get(&(name.to_owned(), entry.to_owned()))
            .map(String::as_str)
    }

    /// The `const enum` names that `ast` also uses as a variable, parameter or loop variable outside its `const
    /// enum` statements; their reads in `ast` aren't replaced.
    #[must_use]
    pub fn variable_names_in(&self, ast: &Ast) -> FxHashSet<String> {
        let enum_assignments: FxHashSet<NodeId> = ast
            .nodes()
            .iter()
            .filter_map(|(_, node)| match &node.kind {
                NodeKind::Enum(enumeration) => Some(enumeration.assignment),
                _ => None,
            })
            .collect();
        let mut variables = FxHashSet::default();

        for (id, node) in ast.nodes().iter() {
            let name = match &node.kind {
                NodeKind::Assignment(assignment) if !enum_assignments.contains(&id) => {
                    match ast.kind(assignment.variable) {
                        NodeKind::Identifier(_) => ast.text(assignment.variable),
                        _ => None,
                    }
                }
                NodeKind::Identifier(identifier)
                    if matches!(
                        identifier.kind,
                        IdentifierKind::Argument | IdentifierKind::ForInVariable
                    ) =>
                {
                    ast.text(id)
                }
                _ => None,
            };

            if let Some(name) = name
                && self.names.contains_key(name)
            {
                variables.insert(name.to_owned());
            }
        }

        variables
    }

    fn add(&mut self, file: usize, ast: &Ast) {
        for (node, data) in ast.nodes().iter() {
            let NodeKind::Enum(enumeration) = &data.kind else {
                continue;
            };
            let NodeKind::Assignment(assignment) = ast.kind(enumeration.assignment) else {
                continue;
            };
            let (true, Some(name), NodeKind::Map(map)) = (
                enumeration.constant,
                ast.text(assignment.variable),
                ast.kind(assignment.init),
            ) else {
                continue;
            };
            let owner = NodeOwner { file, node };

            if *self.names.entry(name.to_owned()).or_insert(owner) != owner {
                self.conflicts.insert(name.to_owned());

                continue;
            }

            for &field in ast.list(map.fields) {
                let NodeKind::MapEntry(entry) = ast.kind(field) else {
                    continue;
                };
                let (NodeKind::Literal(key), NodeKind::Literal(value)) =
                    (ast.kind(entry.key), ast.kind(entry.value))
                else {
                    continue;
                };
                let LiteralValue::String(key) = &key.value else {
                    continue;
                };
                let source = ast.slice(ast.span(entry.value));
                let text = LiteralText::new(value, source, LiteralStyle::Output);
                let text = text
                    .raw_text()
                    .map_or_else(|| text.to_string(), str::to_owned);

                self.entries
                    .insert((name.to_owned(), key.read(ast.source()).into_owned()), text);
            }
        }
    }
}

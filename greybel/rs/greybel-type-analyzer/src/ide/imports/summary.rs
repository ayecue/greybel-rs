use greybel_core::{Ast, LiteralValue, NodeId, NodeKind};
use greybel_documents::SourceFile;
use rustc_hash::FxHashSet;

use crate::analyzer::Analyzer;
use crate::database::Annotate;

#[derive(Default)]
pub(super) struct FileSummary {
    imports: usize,
    namespaces: usize,
    functions: usize,
    variables: usize,
    types: usize,
}

impl FileSummary {
    pub(super) fn new(analyzer: Analyzer<'_>, file: SourceFile) -> Self {
        let db = analyzer.workspace().db();
        let mut summary = Self {
            imports: file.imports(db).dependencies().count(),
            types: file
                .comments(db)
                .blocks
                .iter()
                .map(|block| block.doc.types.len())
                .sum(),
            ..Self::default()
        };
        let Some(ast) = file.parse(db).ast() else {
            return summary;
        };
        let NodeKind::Chunk(chunk) = ast.kind(ast.root()) else {
            return summary;
        };
        let mut targets = FxHashSet::default();

        for &statement in ast.list(chunk.body) {
            let NodeKind::Assignment(assignment) = ast.kind(statement) else {
                continue;
            };

            if !Self::is_member_path(ast, assignment.variable)
                || !targets.insert(ast.slice(ast.span(assignment.variable)))
            {
                continue;
            }

            match ast.kind(assignment.init) {
                NodeKind::Map(_) => summary.namespaces += 1,
                NodeKind::Function(_) => summary.functions += 1,
                _ => summary.variables += 1,
            }
        }

        summary
    }

    fn is_member_path(ast: &Ast, node: NodeId) -> bool {
        match ast.kind(node) {
            NodeKind::Identifier(_) => true,
            NodeKind::Member(member) => Self::is_member_path(ast, member.base),
            NodeKind::Index(index) => {
                let is_name = matches!(
                    ast.kind(index.index),
                    NodeKind::Literal(literal) if matches!(literal.value, LiteralValue::String(_))
                );

                is_name && Self::is_member_path(ast, index.base)
            }
            _ => false,
        }
    }

    pub(super) fn text(&self) -> String {
        let parts: Vec<String> = [
            (self.imports, "import", "imports"),
            (self.namespaces, "namespace", "namespaces"),
            (self.functions, "function", "functions"),
            (self.variables, "variable", "variables"),
            (self.types, "type", "types"),
        ]
        .into_iter()
        .filter(|&(count, _, _)| count > 0)
        .map(|(count, one, many)| format!("{count} {}", if count == 1 { one } else { many }))
        .collect();

        if parts.is_empty() {
            "Contains no imports, assignments or types.".to_owned()
        } else {
            format!("Contains {}.", parts.join(", "))
        }
    }
}

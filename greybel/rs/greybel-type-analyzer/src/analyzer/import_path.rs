use greybel_core::{Ast, ColumnSettings, LiteralValue, NodeId, NodeKind, Position, Range, Span};
use greybel_documents::{Import, SourceFile, Uri};
use rustc_hash::FxHashSet;

use super::Analyzer;
use crate::database::Annotate;
use crate::ide::{Hover, HoverKind, Location};
use crate::model::Documentation;

impl<'w> Analyzer<'w> {
    /// Hover on the path of `#include`, `#include type`, `#import` or `import_code`: the file's path, a count of
    /// what it contains and the description of the comment at its start.
    pub(super) fn import_hover(self, uri: &Uri, position: Position) -> Option<Hover> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let db = self.workspace.db();
        let (import, path) = self.import_at(file, ast, offset)?;
        let range = ast.lines().range(path, ColumnSettings::default());
        let Some(target) = self.import_target(file, import) else {
            return Some(Hover {
                range,
                text: format!("Import statement for {}", import.target),
                kind: HoverKind::Sentence,
                documentation: Some(Documentation {
                    description: Some(format!("Cannot find the file \"{}\".", import.target)),
                    examples: Vec::new(),
                }),
            });
        };
        let mut description = FileSummary::new(self, target).text();

        if let Some(header) = self.file_header(target) {
            description.push_str("\n\n");
            description.push_str(&header);
        }

        Some(Hover {
            range,
            text: format!(
                "Import statement for {}",
                Self::relative_label(file.uri(db), target.uri(db))
            ),
            kind: HoverKind::Sentence,
            documentation: Some(Documentation {
                description: Some(description),
                examples: Vec::new(),
            }),
        })
    }

    /// The start of the file an import path at `position` names, for go to definition.
    pub(super) fn import_definition(self, uri: &Uri, position: Position) -> Option<Location> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (import, _) = self.import_at(file, ast, offset)?;
        let target = self.import_target(file, import)?;
        let db = self.workspace.db();
        let target_ast = target.parse(db).ast()?;

        Some(Location {
            uri: target.uri(db).clone(),
            range: target_ast
                .lines()
                .range(Span::empty(0), ColumnSettings::default()),
        })
    }

    /// The range of the import path at `position`.
    pub(super) fn import_path_range(self, uri: &Uri, position: Position) -> Option<Range> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (_, path) = self.import_at(file, ast, offset)?;

        Some(ast.lines().range(path, ColumnSettings::default()))
    }

    /// The dependency whose path contains `offset`, with the path's span.
    fn import_at(self, file: SourceFile, ast: &Ast, offset: u32) -> Option<(&'w Import, Span)> {
        file.imports(self.workspace.db())
            .dependencies()
            .find_map(|import| {
                let path = Self::import_path(ast, import.node)?;

                (path.start <= offset && offset <= path.end).then_some((import, path))
            })
    }

    fn import_target(self, file: SourceFile, import: &Import) -> Option<SourceFile> {
        let db = self.workspace.db();

        file.links(db)
            .get(&import.target)
            .copied()
            .filter(|target| target.text(db).is_some())
    }

    fn import_path(ast: &Ast, node: NodeId) -> Option<Span> {
        let path = match ast.kind(node) {
            NodeKind::Include(include) => include.path,
            NodeKind::Import(import) => import.path,
            NodeKind::ImportCode(import) => import.original_directory,
            _ => return None,
        };

        Some(path.span())
    }

    /// The description of a comment block that starts on the first line of `target`.
    fn file_header(self, target: SourceFile) -> Option<String> {
        let db = self.workspace.db();
        let ast = target.parse(db).ast()?;
        let block = target.comments(db).blocks.first()?;
        let starts_file = ast.lines().line(block.spans.first()?.start) == 1;

        starts_file.then(|| block.doc.description.clone()).flatten()
    }

    /// The path of `target` relative to the folder of `file`, or the full path when it is outside that folder.
    fn relative_label(file: &Uri, target: &Uri) -> String {
        let path = target.path();
        let folder = file
            .path()
            .rsplit_once('/')
            .map_or("", |(folder, _)| folder);

        path.strip_prefix(folder)
            .and_then(|remainder| remainder.strip_prefix('/'))
            .filter(|_| !folder.is_empty())
            .map_or_else(|| target.label(), str::to_owned)
    }
}

/// Counts of what a file contains at its top level. A target such as `Proxima.utils` assigned several times
/// counts once.
#[derive(Default)]
struct FileSummary {
    imports: usize,
    namespaces: usize,
    functions: usize,
    variables: usize,
    types: usize,
}

impl FileSummary {
    fn new(analyzer: Analyzer<'_>, file: SourceFile) -> Self {
        let db = analyzer.workspace.db();
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

    /// Whether `node` is a variable followed by member names or string-literal indexes.
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

    /// For example "3 imports, 2 namespaces, 12 functions"; counts of 0 are left out.
    fn text(&self) -> String {
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

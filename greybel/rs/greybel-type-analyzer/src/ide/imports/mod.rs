mod path;
mod summary;

use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Range, Span};
use greybel_documents::{Import, SourceFile, Uri};
pub(crate) use path::{ImportTarget, RelativePath};
use summary::FileSummary;

use crate::analyzer::Analyzer;
use crate::database::Annotate;
use crate::ide::{Hover, HoverKind, Location};
use crate::model::Documentation;

impl<'w> Analyzer<'w> {
    pub(crate) fn import_hover(self, uri: &Uri, position: Position) -> Option<Hover> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let db = self.workspace().db();
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

    pub(crate) fn import_definition(self, uri: &Uri, position: Position) -> Option<Location> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (import, _) = self.import_at(file, ast, offset)?;
        let target = self.import_target(file, import)?;
        let db = self.workspace().db();
        let target_ast = target.parse(db).ast()?;

        Some(Location {
            uri: target.uri(db).clone(),
            range: target_ast
                .lines()
                .range(Span::empty(0), ColumnSettings::default()),
        })
    }

    pub(crate) fn import_path_range(self, uri: &Uri, position: Position) -> Option<Range> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let (_, path) = self.import_at(file, ast, offset)?;

        Some(ast.lines().range(path, ColumnSettings::default()))
    }

    fn import_at(self, file: SourceFile, ast: &Ast, offset: u32) -> Option<(&'w Import, Span)> {
        file.imports(self.workspace().db())
            .dependencies()
            .find_map(|import| {
                let path = Self::import_path(ast, import.node)?;

                (path.start <= offset && offset <= path.end).then_some((import, path))
            })
    }

    fn import_target(self, file: SourceFile, import: &Import) -> Option<SourceFile> {
        let db = self.workspace().db();

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

    fn file_header(self, target: SourceFile) -> Option<String> {
        let db = self.workspace().db();
        let ast = target.parse(db).ast()?;
        let block = target.comments(db).blocks.first()?;
        let starts_file = ast.lines().line(block.spans.first()?.start) == 1;

        starts_file.then(|| block.doc.description.clone()).flatten()
    }

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

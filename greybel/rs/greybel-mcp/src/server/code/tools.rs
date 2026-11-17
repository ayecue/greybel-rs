use std::fmt::Write as _;

use greybel_core::{ColumnSettings, LineIndex, Position};
use greybel_documents::Uri;
use greybel_documents_fs_provider::FsProvider;
use greybel_type_analyzer::{CallHierarchyCall, DocumentSymbol, Hover, SymbolKind, WorkspaceEdit};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;

use super::index::{IndexedSymbol, SymbolIndex};
use super::{CodeError, CodeProject, SymbolTarget};
use crate::server::Server;

const SYMBOL_LIMIT: usize = 50;
const REFERENCE_LIMIT: usize = 200;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentSymbolsRequest {
    /// GreyScript file.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindSymbolRequest {
    /// Part of a symbol's name, ignoring case: `form` finds `App.utils.format`. With a dot it matches the name path: `utils.form`.
    pub query: String,
    /// File to search in, or a folder of the project to search; the current folder when absent.
    pub path: Option<String>,
    /// Include each symbol's source code.
    pub include_code: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CallDirection {
    /// Functions that call the function.
    Callers,
    /// Functions the function calls.
    Callees,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CallHierarchyRequest {
    #[serde(flatten)]
    pub target: SymbolTarget,
    pub direction: CallDirection,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameRequest {
    #[serde(flatten)]
    pub target: SymbolTarget,
    /// The new name.
    pub new_name: String,
    /// List the edits without writing the files.
    pub dry_run: Option<bool>,
}

#[tool_router(router = code_router, vis = "pub(in crate::server)")]
impl Server {
    #[tool(
        description = "Lists the symbols of a GreyScript file as a tree: functions, variables, member assignments such as `App.utils.format`, classes and types, with their types and lines."
    )]
    async fn document_symbols(
        &self,
        Parameters(params): Parameters<DocumentSymbolsRequest>,
    ) -> Result<String, CodeError> {
        let project = CodeProject::open(Some(&params.path))?;
        let uri = project.file(&params.path)?;
        let symbols = project.analyzer().document_symbols(&uri);

        if symbols.is_empty() {
            return Ok("The file defines no symbols.".to_owned());
        }

        let mut text = String::new();

        SymbolText::new(&project).write_tree(&uri, &symbols, 0, &mut text);

        Ok(text)
    }

    #[tool(
        description = "Finds GreyScript symbols of a project by name: each with its kind, full name such as `App.utils.format`, type, location and description, and its code when asked."
    )]
    async fn find_symbol(
        &self,
        Parameters(params): Parameters<FindSymbolRequest>,
    ) -> Result<String, CodeError> {
        let project = CodeProject::open(params.path.as_deref())?;
        let file = match params.path.as_deref() {
            Some(path) if std::path::Path::new(path).is_file() => Some(project.file(path)?),
            _ => None,
        };
        let index = SymbolIndex::new(&project);
        let matches = index.search(&params.query, file.as_ref());

        if matches.is_empty() {
            return Ok(format!("No symbol matches \"{}\".", params.query));
        }

        let symbols = SymbolText::new(&project);
        let mut text = String::new();

        for symbol in matches.iter().take(SYMBOL_LIMIT) {
            symbols.write_match(symbol, params.include_code.unwrap_or(false), &mut text);
        }

        if matches.len() > SYMBOL_LIMIT {
            let _ = writeln!(
                text,
                "... {} more; narrow the query or path.",
                matches.len() - SYMBOL_LIMIT
            );
        }

        Ok(text)
    }

    #[tool(
        description = "Lists every use of a GreyScript symbol across the project, with the line it is on. Give the symbol by name, or by path, line and column."
    )]
    async fn find_references(
        &self,
        Parameters(target): Parameters<SymbolTarget>,
    ) -> Result<String, CodeError> {
        let project = target.open_project()?;
        let (uri, position) = target.locate(&project)?;
        let references = project.analyzer().references(&uri, position);

        if references.is_empty() {
            return Ok("No references found.".to_owned());
        }

        let mut text = format!("{} references:\n", references.len());

        for location in references.iter().take(REFERENCE_LIMIT) {
            let _ = writeln!(
                text,
                "{}: {}",
                CodeProject::place(&location.uri, location.range.start),
                project.line_text(&location.uri, location.range.start.line)
            );
        }

        if references.len() > REFERENCE_LIMIT {
            let _ = writeln!(text, "... {} more.", references.len() - REFERENCE_LIMIT);
        }

        Ok(text)
    }

    #[tool(
        description = "Describes a GreyScript symbol as the editor's hover does: its type or signature, its documentation, and where it is defined. Give the symbol by name, or by path, line and column."
    )]
    async fn describe(
        &self,
        Parameters(target): Parameters<SymbolTarget>,
    ) -> Result<String, CodeError> {
        let project = target.open_project()?;
        let (uri, position) = target.locate(&project)?;
        let analyzer = project.analyzer();
        let Some(hover) = analyzer.hover(&uri, position) else {
            return Ok("Nothing is known about the symbol there.".to_owned());
        };
        let mut text = SymbolText::hover_text(&hover);
        let definitions = analyzer.definition(&uri, position);

        if !definitions.is_empty() {
            text.push_str("\n\nDefined at:");

            for location in definitions {
                let _ = write!(
                    text,
                    "\n{}",
                    CodeProject::place(&location.uri, location.range.start)
                );
            }
        }

        Ok(text)
    }

    #[tool(
        description = "Lists the functions that call a GreyScript function (callers), or that it calls (callees), one level deep, with the call sites. Give the function by name, or by path, line and column."
    )]
    async fn call_hierarchy(
        &self,
        Parameters(params): Parameters<CallHierarchyRequest>,
    ) -> Result<String, CodeError> {
        let project = params.target.open_project()?;
        let (uri, position) = params.target.locate(&project)?;
        let analyzer = project.analyzer();
        let Some(item) = analyzer
            .prepare_call_hierarchy(&uri, position)
            .into_iter()
            .next()
        else {
            return Ok("There is no function there.".to_owned());
        };
        let (calls, heading) = match params.direction {
            CallDirection::Callers => (analyzer.incoming_calls(&uri, position), "Called by"),
            CallDirection::Callees => (analyzer.outgoing_calls(&uri, position), "Calls"),
        };
        let mut text = format!(
            "{} {} at {}\n",
            SymbolText::kind_name(item.kind),
            item.name,
            CodeProject::place(&item.uri, item.selection_range.start)
        );

        if calls.is_empty() {
            let _ = write!(text, "{heading}: nothing found.");

            return Ok(text);
        }

        let _ = writeln!(text, "{heading}:");

        for call in &calls {
            SymbolText::write_call(&uri, call, params.direction, &mut text);
        }

        Ok(text)
    }

    #[tool(
        description = "Renames a GreyScript symbol and every use of it across the project, including `@param` tags, and writes the files. Give the symbol by name, or by path, line and column; `dry_run` only lists the edits."
    )]
    async fn rename(
        &self,
        Parameters(params): Parameters<RenameRequest>,
    ) -> Result<String, CodeError> {
        let project = params.target.open_project()?;
        let (uri, position) = params.target.locate(&project)?;
        let analyzer = project.analyzer();

        if analyzer.prepare_rename(&uri, position).is_none() {
            return Err(CodeError::NotRenamable);
        }

        let edit = analyzer
            .rename(&uri, position, &params.new_name)
            .ok_or_else(|| CodeError::InvalidName(params.new_name.clone()))?;
        let count: usize = edit.changes.values().map(Vec::len).sum();
        let is_dry_run = params.dry_run.unwrap_or(false);
        let mut text = format!(
            "{} {count} occurrences in {} files:\n",
            if is_dry_run {
                "Would rename"
            } else {
                "Renamed"
            },
            edit.changes.len()
        );

        for (file, edits) in &edit.changes {
            for change in edits {
                let _ = writeln!(text, "{}", CodeProject::place(file, change.range.start));
            }
        }

        if !is_dry_run {
            EditWriter::new(&project).write(&edit)?;
        }

        Ok(text)
    }
}

/// Formats symbols, their types and calls as tool text.
struct SymbolText<'p> {
    project: &'p CodeProject,
}

impl<'p> SymbolText<'p> {
    const fn new(project: &'p CodeProject) -> Self {
        Self { project }
    }

    fn kind_name(kind: SymbolKind) -> &'static str {
        match kind {
            SymbolKind::Variable => "variable",
            SymbolKind::Function => "function",
            SymbolKind::Property => "property",
            SymbolKind::File => "file",
            SymbolKind::Type => "type",
            SymbolKind::Class => "class",
        }
    }

    /// The first line of the hover text at `position`.
    fn signature(&self, uri: &Uri, position: Position) -> Option<String> {
        let hover = self.project.analyzer().hover(uri, position)?;

        hover.text.lines().next().map(str::to_owned)
    }

    /// `name_path` followed by the part of the hover signature after the symbol's own name.
    fn describe_name(&self, name_path: &str, uri: &Uri, position: Position) -> String {
        let leaf = name_path.rsplit('.').next().unwrap_or(name_path);

        match self.signature(uri, position) {
            Some(signature) => match signature.strip_prefix(leaf) {
                Some(tail) => format!("{name_path}{tail}"),
                None => format!("{name_path} ({signature})"),
            },
            None => name_path.to_owned(),
        }
    }

    fn write_tree(&self, uri: &Uri, symbols: &[DocumentSymbol], depth: usize, text: &mut String) {
        for symbol in symbols {
            let lines = if symbol.range.start.line == symbol.range.end.line {
                format!("line {}", symbol.range.start.line)
            } else {
                format!(
                    "lines {}-{}",
                    symbol.range.start.line, symbol.range.end.line
                )
            };

            let _ = writeln!(
                text,
                "{}{} {}  ({lines})",
                "  ".repeat(depth),
                Self::kind_name(symbol.kind),
                self.describe_name(
                    &symbol.name,
                    uri,
                    IndexedSymbol::leaf_position(
                        symbol.selection_range,
                        symbol.name.rsplit('.').next().unwrap_or(&symbol.name)
                    )
                )
            );

            self.write_tree(uri, &symbol.children, depth + 1, text);
        }
    }

    fn write_match(&self, symbol: &IndexedSymbol, include_code: bool, text: &mut String) {
        let _ = writeln!(
            text,
            "{} {} at {}",
            Self::kind_name(symbol.kind),
            self.describe_name(&symbol.name_path, &symbol.uri, symbol.name_position()),
            CodeProject::place(&symbol.uri, symbol.name_position())
        );

        let description = self
            .project
            .analyzer()
            .hover(&symbol.uri, symbol.name_position())
            .and_then(|hover| hover.documentation)
            .and_then(|documentation| documentation.description);

        if let Some(description) = description.as_deref().and_then(|text| text.lines().next()) {
            let _ = writeln!(text, "  {description}");
        }

        if include_code
            && let Some(code) =
                self.project
                    .slice(&symbol.uri, symbol.range.start, symbol.range.end)
        {
            let _ = writeln!(text, "```greyscript\n{code}\n```");
        }
    }

    fn hover_text(hover: &Hover) -> String {
        let mut text = hover.text.clone();

        if let Some(documentation) = &hover.documentation {
            if let Some(description) = &documentation.description {
                let _ = write!(text, "\n\n{description}");
            }

            for example in &documentation.examples {
                let _ = write!(text, "\n\nExample:\n```greyscript\n{example}\n```");
            }
        }

        text
    }

    fn write_call(
        uri: &Uri,
        call: &CallHierarchyCall,
        direction: CallDirection,
        text: &mut String,
    ) {
        let sites_uri = match direction {
            CallDirection::Callers => &call.item.uri,
            CallDirection::Callees => uri,
        };
        let sites: Vec<String> = call
            .ranges
            .iter()
            .map(|range| CodeProject::place(sites_uri, range.start))
            .collect();

        let _ = writeln!(
            text,
            "  {} {} at {}; calls at {}",
            Self::kind_name(call.item.kind),
            call.item.name,
            CodeProject::place(&call.item.uri, call.item.selection_range.start),
            sites.join(", ")
        );
    }
}

/// Applies a rename's edits to the files on disk.
struct EditWriter<'p> {
    project: &'p CodeProject,
}

impl<'p> EditWriter<'p> {
    const fn new(project: &'p CodeProject) -> Self {
        Self { project }
    }

    fn write(&self, edit: &WorkspaceEdit) -> Result<(), CodeError> {
        for (uri, edits) in &edit.changes {
            let display = FsProvider::display_path(uri);
            let (Some(path), Some(mut text)) = (FsProvider::path(uri), self.project.text(uri))
            else {
                return Err(CodeError::UnknownFile(display));
            };
            let lines = LineIndex::new(&text);
            let mut ranges: Vec<(usize, usize, &str)> = edits
                .iter()
                .filter_map(|edit| {
                    let start = lines.offset(edit.range.start, ColumnSettings::default())? as usize;
                    let end = lines.offset(edit.range.end, ColumnSettings::default())? as usize;

                    Some((start, end, edit.new_text.as_str()))
                })
                .collect();

            ranges.sort_by_key(|&(start, _, _)| std::cmp::Reverse(start));

            for (start, end, new_text) in ranges {
                text.replace_range(start..end, new_text);
            }

            std::fs::write(&path, text).map_err(|error| CodeError::Write {
                path: display,
                error,
            })?;
        }

        Ok(())
    }
}

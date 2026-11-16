use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Range, Span, UnaryOp};
use greybel_documents::Uri;

use super::Analyzer;
use crate::analysis::Analysis;
use crate::ide::{CallHierarchyCall, CallHierarchyItem, SymbolKind};
use crate::model::{DocumentIndex, Site};
use crate::types::FunctionRef;

/// A function node, or the top level of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Caller {
    document: DocumentIndex,
    function: Option<NodeId>,
}

impl<'w> Analyzer<'w> {
    /// The function at `position`: its name in an assignment, or a reference to it.
    #[must_use]
    pub fn prepare_call_hierarchy(&self, uri: &Uri, position: Position) -> Vec<CallHierarchyItem> {
        self.function_at(uri, position)
            .and_then(|(analysis, site)| self.item(analysis, Caller::function(site)))
            .into_iter()
            .collect()
    }

    /// The functions and files that call the function at `position`, with the call sites.
    #[must_use]
    pub fn incoming_calls(&self, uri: &Uri, position: Position) -> Vec<CallHierarchyCall> {
        let Some((analysis, function)) = self.function_at(uri, position) else {
            return Vec::new();
        };
        let Some(variable) = self.assigned_variable(analysis, function) else {
            return Vec::new();
        };
        let references = analysis.references();
        let slots = references.slots(variable).to_vec();
        let mut calls: Vec<(Caller, Vec<Range>)> = Vec::new();

        for site in references.find_referring_sites(&slots) {
            if slots
                .iter()
                .any(|&slot| references.definition_sites(slot).contains(&site))
            {
                continue;
            }

            let Some(ast) = self.document_ast(analysis, site.document) else {
                continue;
            };

            if Self::is_address(ast, site.node) {
                continue;
            }

            let caller = Caller {
                document: site.document,
                function: Self::outer_function(ast, site.node),
            };

            Self::add_call(&mut calls, caller, Self::range(ast, ast.span(site.node)));
        }

        self.calls(analysis, calls)
    }

    /// The functions that the function at `position` calls, with the call sites in its body.
    #[must_use]
    pub fn outgoing_calls(&self, uri: &Uri, position: Position) -> Vec<CallHierarchyCall> {
        let Some((analysis, function)) = self.function_at(uri, position) else {
            return Vec::new();
        };
        let Some(ast) = self.document_ast(analysis, function.document) else {
            return Vec::new();
        };
        let NodeKind::Function(body) = ast.kind(function.node) else {
            return Vec::new();
        };
        let mut nodes = Vec::new();

        for &statement in ast.list(body.body) {
            Self::collect_outside_functions(ast, statement, &mut nodes);
        }

        let mut calls: Vec<(Caller, Vec<Range>)> = Vec::new();

        for node in nodes {
            let site = Site {
                document: function.document,
                node,
            };

            let references = analysis.references();
            let is_definition = references
                .slots(site)
                .iter()
                .any(|&slot| references.definition_sites(slot).contains(&site));

            if is_definition || Self::is_address(ast, node) {
                continue;
            }

            if let Some(callee) = Self::callee(analysis, site) {
                Self::add_call(
                    &mut calls,
                    Caller::function(callee),
                    Self::range(ast, ast.span(node)),
                );
            }
        }

        self.calls(analysis, calls)
    }

    fn function_at(self, uri: &Uri, position: Position) -> Option<(&'w Analysis, Site)> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let &(analysis, document) = self.analyses_of(file).first()?;

        for node in ast.path_at(offset).into_iter().rev() {
            if let NodeKind::Assignment(assignment) = ast.kind(node) {
                let variable = ast.span(assignment.variable);

                if variable.start <= offset
                    && offset <= variable.end
                    && matches!(ast.kind(assignment.init), NodeKind::Function(_))
                {
                    return Some((
                        analysis,
                        Site {
                            document,
                            node: assignment.init,
                        },
                    ));
                }
            }

            if let Some(callee) = Self::callee(analysis, Site { document, node }) {
                return Some((analysis, callee));
            }
        }

        None
    }

    /// The function node of a script function that the variable or member at `site` contains.
    fn callee(analysis: &Analysis, site: Site) -> Option<Site> {
        let model = analysis.model();

        analysis.references().slots(site).iter().find_map(|&slot| {
            let ty = model.known_slot_type(slot)?;

            analysis
                .types()
                .function_refs(ty)
                .find_map(|function| match function {
                    FunctionRef::Body(id) => Some(model.function(id).site),
                    _ => None,
                })
        })
    }

    /// The variable or member that `function` is assigned to.
    fn assigned_variable(self, analysis: &Analysis, function: Site) -> Option<Site> {
        let ast = self.document_ast(analysis, function.document)?;
        let start = ast.span(function.node).start;

        ast.path_at(start)
            .into_iter()
            .find_map(|node| match ast.kind(node) {
                NodeKind::Assignment(assignment) if assignment.init == function.node => {
                    Some(Site {
                        document: function.document,
                        node: assignment.variable,
                    })
                }
                _ => None,
            })
    }

    fn item(self, analysis: &Analysis, caller: Caller) -> Option<CallHierarchyItem> {
        let file = *analysis.documents().get(caller.document as usize)?;
        let db = self.workspace.db();
        let ast = file.parse(db).ast()?;
        let uri = file.uri(db).clone();

        let Some(function) = caller.function else {
            let end = u32::try_from(ast.source().len()).ok()?;

            return Some(CallHierarchyItem {
                name: uri.label(),
                kind: SymbolKind::File,
                range: ast
                    .lines()
                    .range(Span::new(0, end), ColumnSettings::default()),
                selection_range: ast.lines().range(Span::empty(0), ColumnSettings::default()),
                uri,
            });
        };
        let variable = self.assigned_variable(
            analysis,
            Site {
                document: caller.document,
                node: function,
            },
        );
        let (name, selection) = match variable {
            Some(variable) => (
                ast.slice(ast.span(variable.node)).to_owned(),
                Self::range(ast, ast.span(variable.node)),
            ),
            None => ("function".to_owned(), Self::range(ast, ast.span(function))),
        };

        Some(CallHierarchyItem {
            name,
            kind: SymbolKind::Function,
            range: Self::range(ast, ast.span(function)),
            selection_range: selection,
            uri,
        })
    }

    fn calls(
        self,
        analysis: &Analysis,
        calls: Vec<(Caller, Vec<Range>)>,
    ) -> Vec<CallHierarchyCall> {
        calls
            .into_iter()
            .filter_map(|(caller, ranges)| {
                Some(CallHierarchyCall {
                    item: self.item(analysis, caller)?,
                    ranges,
                })
            })
            .collect()
    }

    fn add_call(calls: &mut Vec<(Caller, Vec<Range>)>, caller: Caller, range: Range) {
        match calls.iter_mut().find(|(entry, _)| *entry == caller) {
            Some((_, ranges)) => ranges.push(range),
            None => calls.push((caller, vec![range])),
        }
    }

    fn document_ast(self, analysis: &Analysis, document: DocumentIndex) -> Option<&'w Ast> {
        analysis
            .documents()
            .get(document as usize)?
            .parse(self.workspace.db())
            .ast()
            .map(|ast| &**ast)
    }

    /// Identifiers and members in `node`, not entering function literals.
    fn collect_outside_functions(ast: &Ast, node: NodeId, nodes: &mut Vec<NodeId>) {
        match ast.kind(node) {
            NodeKind::Function(_) => return,
            NodeKind::Identifier(_) | NodeKind::Member(_) | NodeKind::Index(_) => nodes.push(node),
            _ => {}
        }

        ast.for_each_child(node, |child| {
            Self::collect_outside_functions(ast, child, nodes)
        });
    }

    fn outer_function(ast: &Ast, node: NodeId) -> Option<NodeId> {
        ast.path_at(ast.span(node).start)
            .into_iter()
            .rev()
            .find(|&candidate| {
                candidate != node && matches!(ast.kind(candidate), NodeKind::Function(_))
            })
    }

    /// Whether `node` is the operand of `@`, which reads the function without calling it.
    fn is_address(ast: &Ast, node: NodeId) -> bool {
        ast.path_at(ast.span(node).start).windows(2).any(|pair| {
            pair[1] == node
                && matches!(
                    ast.kind(pair[0]),
                    NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf
                )
        })
    }
}

impl Caller {
    const fn function(site: Site) -> Self {
        Self {
            document: site.document,
            function: Some(site.node),
        }
    }
}

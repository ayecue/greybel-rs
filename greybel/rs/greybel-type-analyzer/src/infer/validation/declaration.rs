use greybel_core::syntax::Declaration;
use greybel_core::syntax::greybel::DestructuringAssignment;
use greybel_core::{Ast, NodeId, NodeKind};
use indexmap::IndexMap;

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::model::{DocumentIndex, Site, SlotId};

struct NamedAssignment {
    site: Site,
    slot: SlotId,
    declaration: Option<Declaration>,
}

impl Inference<'_> {
    pub(crate) fn validate_declarations(&mut self) {
        let assignments = self.named_assignments();
        let mut declarations: IndexMap<SlotId, Vec<(Site, Declaration)>> = IndexMap::new();
        let mut reports: Vec<(Site, DiagnosticKind, String)> = Vec::new();

        for assignment in &assignments {
            if let Some(declaration) = assignment.declaration {
                declarations
                    .entry(assignment.slot)
                    .or_default()
                    .push((assignment.site, declaration));
            }
        }

        for (&slot, sites) in &declarations {
            let name = self.site_name(sites[0].0);

            for &(site, declaration) in &sites[1..] {
                let keyword = declaration.as_str();

                reports.push((
                    site,
                    DiagnosticKind::DuplicateDeclaration,
                    format!("\"{name}\" is already declared; remove \"{keyword}\" here."),
                ));
            }

            let Some(&(constant, _)) = sites
                .iter()
                .find(|(_, declaration)| *declaration == Declaration::Const)
            else {
                continue;
            };

            for &site in self.references.definition_sites(slot) {
                if site != constant {
                    reports.push((
                        site,
                        DiagnosticKind::ConstAssignment,
                        format!("\"{name}\" is a constant; it can't be assigned again."),
                    ));
                }
            }
        }

        for assignment in &assignments {
            if assignment.declaration.is_some() || declarations.contains_key(&assignment.slot) {
                continue;
            }

            let ast = self.documents[assignment.site.document as usize].ast;
            let first = self
                .references
                .definition_sites(assignment.slot)
                .iter()
                .filter(|site| site.document == assignment.site.document)
                .min_by_key(|site| ast.span(site.node).start);

            if first == Some(&assignment.site) {
                let name = self.site_name(assignment.site);

                reports.push((
                    assignment.site,
                    DiagnosticKind::MissingLet,
                    format!("\"{name}\" is assigned without \"let\" or \"const\"."),
                ));
            }
        }

        for (site, kind, message) in reports {
            self.report_in(site.document, kind, site.node, message);
        }
    }

    fn site_name(&self, site: Site) -> String {
        let ast = self.documents[site.document as usize].ast;

        ast.name(site.node).unwrap_or_default().to_owned()
    }

    fn named_assignments(&self) -> Vec<NamedAssignment> {
        let mut assignments = Vec::new();

        for (index, document) in self.documents.iter().enumerate() {
            let document_index = DocumentIndex::try_from(index).expect("fewer than 2^32 documents");
            let mut statements = Vec::new();

            Self::statement_assignments(document.ast, document.ast.root(), &mut statements);

            for (variable, declaration) in statements {
                let site = Site {
                    document: document_index,
                    node: variable,
                };

                let temporary = document.ast.name(variable).is_some_and(|name| {
                    name.starts_with(DestructuringAssignment::TEMPORARY_PREFIX)
                });

                if !temporary && let &[slot] = self.references.slots(site) {
                    assignments.push(NamedAssignment {
                        site,
                        slot,
                        declaration,
                    });
                }
            }
        }

        assignments
    }

    fn statement_assignments(
        ast: &Ast,
        node: NodeId,
        assignments: &mut Vec<(NodeId, Option<Declaration>)>,
    ) {
        let body = match ast.kind(node) {
            NodeKind::Chunk(chunk) => chunk.body,
            NodeKind::Function(function) => function.body,
            NodeKind::IfClause(clause) => clause.body,
            NodeKind::ElseClause(clause) => clause.body,
            NodeKind::While(statement) => statement.body,
            NodeKind::For(statement) => statement.body,
            NodeKind::If(statement) => statement.clauses,
            NodeKind::Assignment(assignment) => {
                if matches!(ast.kind(assignment.variable), NodeKind::Identifier(_)) {
                    assignments.push((assignment.variable, assignment.declaration));
                }

                return Self::statement_assignments(ast, assignment.init, assignments);
            }
            _ => {
                ast.for_each_child(node, |child| {
                    Self::statement_assignments(ast, child, assignments);
                });

                return;
            }
        };

        for &statement in ast.list(body) {
            Self::statement_assignments(ast, statement, assignments);
        }
    }
}

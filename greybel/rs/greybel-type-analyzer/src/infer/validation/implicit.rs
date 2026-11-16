use greybel_core::{NodeId, NodeKind};
use greybel_symbols::names::SELF;

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::model::DocumentIndex;
use crate::types::TypeId;

impl Inference<'_> {
    pub(crate) fn validate_implicit_any(&mut self) {
        let mut reports: Vec<(DocumentIndex, NodeId, String)> = Vec::new();

        for id in self.model.functions() {
            let data = self.model.function(id);
            let (document, node) = (data.site.document, data.site.node);
            let ast = self.documents[document as usize].ast;

            if !matches!(ast.kind(node), NodeKind::Function(_)) {
                continue;
            }

            let doc = data.doc.as_deref();

            for parameter in &data.parameters {
                let has_type = matches!(ast.kind(parameter.node), NodeKind::Assignment(_))
                    || parameter.name == SELF
                    || doc.is_some_and(|doc| {
                        doc.params
                            .iter()
                            .any(|param| param.name == parameter.name && param.ty.is_some())
                    });

                if !has_type {
                    reports.push((
                        document,
                        parameter.node,
                        format!("The type of \"{}\" is implicitly any.", parameter.name),
                    ));
                }
            }

            let returns = self.model.slot_type(data.return_slot);

            if doc.is_none_or(|doc| doc.returns.is_none())
                && (returns == TypeId::ANY || returns == TypeId::UNKNOWN)
            {
                reports.push((
                    document,
                    node,
                    "The return type is implicitly any.".to_owned(),
                ));
            }
        }

        for (index, source) in self.documents.iter().enumerate() {
            let ast = source.ast;
            let document =
                DocumentIndex::try_from(index).expect("value fits the target integer type");

            for (_, node) in ast.nodes().iter() {
                let NodeKind::Assignment(assignment) = &node.kind else {
                    continue;
                };
                let value = self.node_types[index]
                    .get(assignment.init.index())
                    .copied()
                    .flatten();

                if !matches!(value, Some(TypeId::ANY | TypeId::UNKNOWN))
                    || !matches!(ast.kind(assignment.variable), NodeKind::Identifier(_))
                    || ast.type_syntax.variable(assignment.variable).is_some()
                {
                    continue;
                }

                let Some(name) = ast.text(assignment.variable) else {
                    continue;
                };

                reports.push((
                    document,
                    assignment.variable,
                    format!("The type of \"{name}\" is implicitly any."),
                ));
            }
        }

        for (document, node, message) in reports {
            self.report_in(document, DiagnosticKind::ImplicitAny, node, message);
        }
    }
}

use super::Context;
use crate::model::Documentation;
use crate::query::Query;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub parameters: Vec<ParameterInformation>,
    pub active_parameter: usize,
    pub documentation: Option<Documentation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInformation {
    pub label: String,
    pub documentation: Option<String>,
}

impl Context<'_> {
    pub(crate) fn signature_help(&self) -> Option<SignatureHelp> {
        let text = self.source_text();
        let (callee_end, active_parameter) = text.open_call(self.offset as usize)?;
        let mut query = Query::new(self.analysis);
        let callee = self
            .typed_node_at_end(callee_end as u32, |node| self.is_reference(node))
            .map(|(_, ty)| ty)
            .or_else(|| self.chain_type(&mut query, callee_end as u32, false))?;
        let name_start = text.name_start(callee_end);
        let name = &self.ast.source()[name_start..callee_end];
        let method = self.ast.source()[..name_start].ends_with('.');
        let function = query.types().function_refs(callee).next()?;
        let model = self.analysis.model();
        let signature = model.signature(function, method)?;
        let notation = self.analysis.notation_in(query.types());
        let parameters = signature
            .parameters
            .iter()
            .map(|parameter| ParameterInformation {
                label: notation.format_parameter(parameter),
                documentation: parameter.description.map(str::to_owned),
            })
            .collect();

        Some(SignatureHelp {
            label: format!("{name}{}", notation.format_signature(&signature)),
            parameters,
            active_parameter,
            documentation: model.documentation(self.analysis.api(), function),
        })
    }
}

use super::compare::NameComparison;
use super::{Context, WorkspaceEdit};
use crate::analysis::Query;
use crate::diagnostic::Diagnostic;
use crate::model::ScopeId;
use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub diagnostic: Diagnostic,
    pub edit: WorkspaceEdit,
    pub is_preferred: bool,
}

impl Context<'_> {
    /// The completions at the offset closest to `name` (variables and functions, or members after a `.`), together
    /// with `extra` candidates.
    pub(crate) fn closest_completions(&self, name: &str, extra: &[&str]) -> Vec<String> {
        let items = self.completions();
        let candidates = items
            .iter()
            .map(|item| item.label.as_str())
            .chain(extra.iter().copied());

        NameComparison::new(name)
            .closest(candidates)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// Members the script assigns on `self` in the enclosing function: `name` itself first, then names close to it.
    pub(crate) fn receiver_members(&self, name: &str) -> Vec<String> {
        let ScopeId::Function(function) = self.scope() else {
            return Vec::new();
        };
        let model = self.analysis.model();
        let receiver = model.slot_type(model.function(function).receiver_slot);
        let mut query = Query::new(self.analysis);
        let is_script_map = query
            .types()
            .variants(receiver)
            .any(|variant| matches!(query.types().get(variant), Type::Object(_)));

        if !is_script_map {
            return Vec::new();
        }

        let members = query.members(receiver);
        let names: Vec<&str> = members
            .iter()
            .filter(|member| !member.from_api)
            .map(|member| member.name.as_str())
            .collect();
        let mut matches: Vec<String> = Vec::new();

        if names.contains(&name) {
            matches.push(name.to_owned());
        }

        matches.extend(
            NameComparison::new(name)
                .closest(names)
                .into_iter()
                .map(str::to_owned),
        );

        matches
    }

    pub(crate) fn is_visible(&self, name: &str) -> bool {
        self.analysis
            .model()
            .resolve_variable(self.scope(), name)
            .is_some()
    }
}

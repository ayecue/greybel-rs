use greybel_symbols::Keyword;

use super::{CodeAction, FixContext, QuickFix};
use crate::analysis::Query;
use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{Context, NameComparison, TextEdit};
use crate::model::ScopeId;
use crate::types::Type;

pub(super) struct NameSuggestion;

impl QuickFix for NameSuggestion {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        matches!(
            kind,
            DiagnosticKind::UnresolvedReference | DiagnosticKind::UnknownMember
        )
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let Some(name) = Analyzer::text_in(context.ast, diagnostic.range) else {
            return Vec::new();
        };
        let Some(ide_context) = context
            .analyzer
            .ide_context(context.uri, diagnostic.range.end)
        else {
            return Vec::new();
        };
        let mut replacements: Vec<String> = Vec::new();

        if diagnostic.kind == DiagnosticKind::UnresolvedReference {
            replacements.extend(
                ide_context
                    .receiver_members(name)
                    .into_iter()
                    .map(|member| format!("self.{member}")),
            );
        }

        let source = context.ast.source();
        let start = context.offset(diagnostic.range.start).unwrap_or_default() as usize;
        let line_start = source[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let starts_statement = diagnostic.kind == DiagnosticKind::UnresolvedReference
            && source[line_start..start].trim().is_empty();
        let keywords: &[&str] = if starts_statement {
            &[
                Keyword::Return.as_str(),
                Keyword::Break.as_str(),
                Keyword::Continue.as_str(),
            ]
        } else {
            &[]
        };

        replacements.extend(ide_context.closest_completions(name, keywords));

        replacements
            .into_iter()
            .map(|replacement| {
                let edit = TextEdit {
                    range: diagnostic.range,
                    new_text: replacement.clone(),
                };

                context.action(
                    diagnostic,
                    format!("Change to \"{replacement}\""),
                    vec![edit],
                )
            })
            .collect()
    }
}

impl Context<'_> {
    fn closest_completions(&self, name: &str, extra: &[&str]) -> Vec<String> {
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

    fn receiver_members(&self, name: &str) -> Vec<String> {
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
}

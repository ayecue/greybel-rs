use std::sync::OnceLock;

use greybel_core::Dialect;
use meta_utils::BaseType;
use rustc_hash::FxHashSet;

use super::{Context, TextEdit};
use crate::api::Api;
use crate::lookup::Lookup;
use crate::model::{Documentation, ScopeId, SemanticModel};
use crate::query::Query;
use crate::types::{TypeId, TypeNotation, TypeTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: String,
    pub documentation: Option<Documentation>,
    pub text_edit: Option<TextEdit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionItemKind {
    Variable,
    Function,
    Property,
    Method,
    File,
}

impl CompletionItem {
    fn new(query: &Query<'_>, label: String, ty: TypeId, kind: CompletionItemKind) -> Self {
        Self {
            label,
            kind,
            detail: query.display(ty).to_string(),
            documentation: query.documentation(ty),
            text_edit: None,
        }
    }

    fn general_completions(api: Api) -> &'static [Self] {
        static GENERAL: [OnceLock<Vec<CompletionItem>>; 3] =
            [OnceLock::new(), OnceLock::new(), OnceLock::new()];
        let slot = match api.dialect() {
            Dialect::MiniScript => 0,
            Dialect::Greybel => 1,
            Dialect::GreyScript => 2,
        };

        GENERAL[slot].get_or_init(|| Self::build_general_completions(api))
    }

    fn build_general_completions(api: Api) -> Vec<Self> {
        let model = SemanticModel::default();
        let mut types = TypeTable::default();
        let members: Vec<(&'static str, TypeId)> = api
            .members(BaseType::General.as_str())
            .filter_map(|member| {
                let name = member.name();

                Some((
                    name,
                    Lookup::new(&mut types, &model, api).general_function_type(name)?,
                ))
            })
            .collect();
        let notation = TypeNotation::new(&model, &types);

        members
            .into_iter()
            .map(|(name, ty)| Self {
                label: name.to_owned(),
                kind: Self::completion_kind(&types, ty),
                detail: notation.display(ty).to_string(),
                documentation: match types.get(ty) {
                    crate::types::Type::Function(function) => model.documentation(api, *function),
                    _ => None,
                },
                text_edit: None,
            })
            .collect()
    }

    fn completion_kind(types: &TypeTable, ty: TypeId) -> CompletionItemKind {
        if types.function_refs(ty).next().is_some() {
            CompletionItemKind::Function
        } else {
            CompletionItemKind::Variable
        }
    }
}

impl Context<'_> {
    pub(crate) fn completions(&self) -> Vec<CompletionItem> {
        let start = self.source_text().name_start(self.offset as usize);
        let mut query = Query::new(self.analysis);

        if self.ast.source()[..start].ends_with('.') {
            let dot = (start - 1) as u32;
            let Some(receiver) = self.receiver_type(&mut query, dot) else {
                return Vec::new();
            };

            return query
                .members(receiver)
                .into_iter()
                .map(|member| {
                    let kind = if query.types().function_refs(member.ty).next().is_some() {
                        CompletionItemKind::Method
                    } else {
                        CompletionItemKind::Property
                    };

                    CompletionItem::new(&query, member.name, member.ty, kind)
                })
                .collect();
        }

        let model = self.analysis.model();
        let mut labels = FxHashSet::default();
        let mut items: Vec<CompletionItem> = Vec::new();

        for (name, slot) in model
            .visible_scopes(self.scope())
            .flat_map(|scope: ScopeId| model.variables(scope))
        {
            if !labels.insert(name) {
                continue;
            }

            let ty = model.slot_type(slot);
            let kind = CompletionItem::completion_kind(query.types(), ty);

            items.push(CompletionItem::new(&query, name.to_owned(), ty, kind));
        }

        let general = CompletionItem::general_completions(self.analysis.api());

        items.extend(
            general
                .iter()
                .filter(|item| !labels.contains(item.label.as_str()))
                .cloned(),
        );

        items
    }

    fn receiver_type(&self, query: &mut Query<'_>, dot: u32) -> Option<TypeId> {
        if let Some((node, ty)) = self.typed_node_at_end(dot, |_| true) {
            return Some(if self.is_reference(node) {
                query.invocation_type(ty)
            } else {
                ty
            });
        }

        self.chain_type(query, dot, true)
    }
}

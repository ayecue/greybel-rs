use std::sync::OnceLock;

use greybel_core::{ColumnSettings, Dialect, Span};
use meta_utils::BaseType;
use rustc_hash::FxHashSet;

use super::{CompletionItem, CompletionItemKind};
use crate::analysis::Query;
use crate::api::Api;
use crate::ide::{Context, TextEdit};
use crate::lookup::Lookup;
use crate::model::{ScopeId, SemanticModel};
use crate::types::{TypeId, TypeNotation, TypeTable};

impl CompletionItem {
    fn new(query: &Query<'_>, label: String, ty: TypeId, kind: CompletionItemKind) -> Self {
        Self {
            label,
            kind,
            detail: query.display(ty).to_string(),
            documentation: query.documentation(ty),
            text_edit: None,
            snippet: None,
            deprecated: query.is_deprecated(ty),
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
                snippet: None,
                deprecated: false,
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

        if let Some(keys) = self.key_completions(&mut query) {
            return keys;
        }

        if self.ast.source()[..start].ends_with('.') {
            let dot = u32::try_from(start - 1).expect("value fits the target integer type");
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
        items.extend(self.keyword_completions());

        items
    }

    fn key_completions(&self, query: &mut Query<'_>) -> Option<Vec<CompletionItem>> {
        let source = self.ast.source();
        let offset = self.offset as usize;
        let line_start = source[..offset]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let quote = line_start + source[line_start..offset].rfind('"')?;
        let before_quote = source[..quote].trim_end();
        let bracket = before_quote.strip_suffix('[')?.len();

        if source[line_start..quote].matches('"').count() % 2 == 1 {
            return None;
        }

        let receiver = self.receiver_type(query, u32::try_from(bracket).ok()?)?;
        let range = self.ast.lines().range(
            Span::new(u32::try_from(quote + 1).ok()?, self.offset),
            ColumnSettings::default(),
        );
        let items = query
            .members(receiver)
            .into_iter()
            .filter(|member| !member.from_api)
            .map(|member| {
                let mut item = CompletionItem::new(
                    query,
                    member.name,
                    member.ty,
                    CompletionItemKind::Property,
                );

                item.text_edit = Some(TextEdit {
                    range,
                    new_text: item.label.clone(),
                });

                item
            })
            .collect();

        Some(items)
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

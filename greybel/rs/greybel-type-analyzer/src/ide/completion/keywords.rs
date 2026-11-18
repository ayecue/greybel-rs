use greybel_core::NodeKind;
use greybel_symbols::Keyword;

use super::{CompletionItem, CompletionItemKind};
use crate::ide::Context;
use crate::ide::text::SourceText;

const STATEMENT_SNIPPETS: &[(&str, &str, &str)] = &[
    (
        "if",
        "if ... then ... end if",
        "if ${1:condition} then\n\t$0\nend if",
    ),
    (
        "if else",
        "if ... then ... else ... end if",
        "if ${1:condition} then\n\t$2\nelse\n\t$0\nend if",
    ),
    (
        "for",
        "for ... in ... end for",
        "for ${1:item} in ${2:list}\n\t$0\nend for",
    ),
    (
        "while",
        "while ... end while",
        "while ${1:condition}\n\t$0\nend while",
    ),
    (
        "function",
        "name = function ... end function",
        "${1:name} = function(${2})\n\t$0\nend function",
    ),
];

const EXPRESSION_SNIPPETS: &[(&str, &str, &str)] = &[(
    "function",
    "function ... end function",
    "function(${1})\n\t$0\nend function",
)];

const VALUE_KEYWORDS: &[&str] = &["true", "false", "null"];

impl Context<'_> {
    pub(super) fn keyword_completions(&self) -> Vec<CompletionItem> {
        let source = self.ast.source();
        let start = self.source_text().name_start(self.offset as usize);
        let line_start = source[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let before = source[line_start..start].trim_end();
        let mut items = Vec::new();

        let statement_start = before.is_empty()
            || [Keyword::Then, Keyword::Else].iter().any(|keyword| {
                before.ends_with(keyword.as_str()) && !Self::ends_with_operand(before)
            });

        if statement_start {
            for &(label, detail, snippet) in STATEMENT_SNIPPETS {
                items.push(CompletionItem::snippet(label, detail, snippet));
            }

            let mut keywords = vec![Keyword::Return, Keyword::Else, Keyword::ElseIf];

            if self.is_in_loop() {
                keywords.extend([Keyword::Break, Keyword::Continue]);
            }

            keywords.extend(self.open_block_end());
            items.extend(
                keywords
                    .into_iter()
                    .map(|keyword| CompletionItem::keyword(keyword.as_str())),
            );

            return items;
        }

        if Self::ends_with_operand(before) {
            return [Keyword::And, Keyword::Or, Keyword::Isa]
                .into_iter()
                .map(|keyword| CompletionItem::keyword(keyword.as_str()))
                .collect();
        }

        for &(label, detail, snippet) in EXPRESSION_SNIPPETS {
            items.push(CompletionItem::snippet(label, detail, snippet));
        }

        items.extend(
            VALUE_KEYWORDS
                .iter()
                .copied()
                .chain([Keyword::New.as_str(), Keyword::Not.as_str()])
                .map(CompletionItem::keyword),
        );

        items
    }

    fn ends_with_operand(before: &str) -> bool {
        if before.ends_with([')', ']', '}', '"']) {
            return true;
        }

        let word = &before[SourceText::new(before).name_start(before.len())..];

        !word.is_empty() && Keyword::parse(word).is_none()
    }

    fn is_in_loop(&self) -> bool {
        for node in self.ast.path_at(self.offset).into_iter().rev() {
            match self.ast.kind(node) {
                NodeKind::While(_) | NodeKind::For(_) => return true,
                NodeKind::Function(_) => return false,
                _ => {}
            }
        }

        false
    }

    fn open_block_end(&self) -> Option<Keyword> {
        self.ast
            .path_at(self.offset)
            .into_iter()
            .rev()
            .find_map(|node| match self.ast.kind(node) {
                NodeKind::Function(_) => Some(Keyword::EndFunction),
                NodeKind::While(_) => Some(Keyword::EndWhile),
                NodeKind::For(_) => Some(Keyword::EndFor),
                NodeKind::IfClause(clause) if !clause.shortcut => Some(Keyword::EndIf),
                NodeKind::ElseClause(clause) if !clause.shortcut => Some(Keyword::EndIf),
                _ => None,
            })
    }
}

impl CompletionItem {
    fn keyword(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            kind: CompletionItemKind::Keyword,
            detail: String::new(),
            documentation: None,
            text_edit: None,
            snippet: None,
            deprecated: false,
        }
    }

    fn snippet(label: &str, detail: &str, snippet: &str) -> Self {
        Self {
            label: label.to_owned(),
            kind: CompletionItemKind::Snippet,
            detail: detail.to_owned(),
            documentation: None,
            text_edit: None,
            snippet: Some(snippet.to_owned()),
            deprecated: false,
        }
    }
}

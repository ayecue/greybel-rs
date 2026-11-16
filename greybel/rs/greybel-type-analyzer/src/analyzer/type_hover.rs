use greybel_core::{Ast, ColumnSettings, Position, Span};
use greybel_documents::Uri;
use greybel_symbols::annotations::{ANY, BOOL, BOOLEAN, NEVER, TAG_PREFIX, Tag, UNKNOWN};
use meta_utils::TypeLookup;

use super::Analyzer;
use crate::analysis::Analysis;
use crate::ide::{Hover, HoverKind};
use crate::model::Documentation;

impl Analyzer<'_> {
    /// Hover on a type name, in code (`port: number`, `type Id = ...`, `as T`, `implements T`) or in a doc comment
    /// (`{T}`, `@type T`, `@vtype T`, `@typedef {..} T`, `@extends T`): what the type is.
    pub(super) fn type_hover(self, uri: &Uri, position: Position) -> Option<Hover> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let span = Self::type_name_at(ast, offset)?;
        let (analysis, _) = self.analyses_of(file).into_iter().next()?;
        let (text, description) = Self::describe_type(analysis, ast.slice(span))?;

        Some(Hover {
            range: ast.lines().range(span, ColumnSettings::default()),
            text,
            kind: HoverKind::Signature,
            documentation: description.map(|description| Documentation {
                description: Some(description),
                examples: Vec::new(),
            }),
        })
    }

    fn describe_type(analysis: &Analysis, name: &str) -> Option<(String, Option<String>)> {
        let model = analysis.model();
        let notation = analysis.notation_in(analysis.types());

        if let Some(custom) = model.custom_type(name) {
            let parameters = if custom.type_parameters.is_empty() {
                String::new()
            } else {
                format!("<{}>", custom.type_parameters.join(", "))
            };
            let text = match notation.custom_type_content(custom.map) {
                Some(content) => format!("type {name}{parameters} {content}"),
                None => format!("type {name}{parameters}"),
            };

            return Some((text, custom.description.clone()));
        }

        if let Some(ty) = analysis.named_type(name) {
            return Some((format!("type {name} = {}", notation.display(ty)), None));
        }

        let is_known = !matches!(analysis.api().lookup_type(name), TypeLookup::Unknown)
            || [ANY, UNKNOWN, NEVER, BOOL, BOOLEAN].contains(&name);

        is_known.then(|| (format!("type {name}"), None))
    }

    /// The span of the type name at `offset` when `offset` is in a place that contains a type.
    fn type_name_at(ast: &Ast, offset: u32) -> Option<Span> {
        let source = ast.source().as_bytes();
        let is_name = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
        let mut start = offset as usize;
        let mut end = offset as usize;

        while start > 0 && is_name(source[start - 1]) {
            start -= 1;
        }

        while end < source.len() && is_name(source[end]) {
            end += 1;
        }

        if start == end || source[start].is_ascii_digit() {
            return None;
        }

        let span = Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?);
        let contains = |outer: Span| outer.start <= span.start && span.end <= outer.end;

        let in_code = ast.type_spans().into_iter().any(contains);
        let in_comment = ast.comments.iter().any(|comment| {
            contains(comment.span) && Self::is_comment_type(ast, comment.span, span)
        });

        (in_code || in_comment).then_some(span)
    }

    /// Whether `name` in the comment at `comment` is a type: inside `{...}` of its line, or the name after `@type`,
    /// `@vtype`, `@extends` or `@typedef {...}`.
    fn is_comment_type(ast: &Ast, comment: Span, name: Span) -> bool {
        let source = ast.source();
        let line_start = source[comment.start as usize..name.start as usize]
            .rfind('\n')
            .map_or(comment.start as usize, |index| {
                comment.start as usize + index + 1
            });
        let line_end = source[name.end as usize..comment.end as usize]
            .find('\n')
            .map_or(comment.end as usize, |index| name.end as usize + index);
        let before = &source[line_start..name.start as usize];
        let after = &source[name.end as usize..line_end];

        if before
            .rfind('{')
            .is_some_and(|open| before.rfind('}').is_none_or(|close| close < open))
            && after.contains('}')
        {
            return true;
        }

        let Some(tag) = before.rfind(TAG_PREFIX).map(|at| &before[at + 1..]) else {
            return false;
        };
        let (tag, tail) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));

        match Tag::parse(tag) {
            Some(Tag::Type | Tag::VirtualType | Tag::Extends) => tail.trim().is_empty(),
            Some(Tag::Typedef) => tail.trim_end().ends_with('}'),
            _ => false,
        }
    }
}

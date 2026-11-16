use greybel_core::Dialect;
use greybel_documents::{ImportKind, Uri};
use greybel_symbols::Keyword;
use greybel_symbols::contextual_keywords::{FROM, TYPE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportTarget<'a> {
    pub kind: ImportKind,
    pub prefix: &'a str,
    pub start: usize,
}

impl<'a> ImportTarget<'a> {
    pub(crate) fn at(source: &'a str, offset: usize, dialect: Dialect) -> Option<Self> {
        if dialect == Dialect::MiniScript {
            return None;
        }

        let line_start = source[..offset].rfind(['\n', '\r']).map_or(0, |at| at + 1);
        let line = &source[line_start..offset];

        if line.matches('"').count().is_multiple_of(2) {
            return None;
        }

        let quote = line.rfind('"')?;
        let before = line[..quote].trim();
        let kind = if before == Keyword::Include.as_str() {
            ImportKind::Include
        } else if Self::is_type_include(before) {
            ImportKind::TypeInclude
        } else if Self::is_import(before) {
            ImportKind::Import
        } else if dialect == Dialect::GreyScript
            && before
                .strip_suffix('(')
                .is_some_and(|callee| callee.trim_end().ends_with(Keyword::ImportCode.as_str()))
        {
            ImportKind::NativeImport
        } else {
            return None;
        };

        Some(Self {
            kind,
            prefix: &line[quote + 1..],
            start: line_start + quote + 1,
        })
    }

    fn is_type_include(before: &str) -> bool {
        const INCLUDE: &str = Keyword::Include.as_str();

        let words: Vec<&str> = before.split_whitespace().collect();

        matches!(words[..], [INCLUDE, TYPE])
    }

    fn is_import(before: &str) -> bool {
        const IMPORT: &str = Keyword::Import.as_str();

        let words: Vec<&str> = before.split_whitespace().collect();

        matches!(words[..], [IMPORT, _, FROM])
    }
}

pub(crate) struct RelativePath;

impl RelativePath {
    pub(crate) fn between(from_path: &Uri, to_path: &Uri) -> Option<String> {
        let same_root =
            from_path.scheme() == to_path.scheme() && from_path.authority() == to_path.authority();

        same_root.then(|| Self::between_paths(from_path.path(), to_path.path()))
    }

    fn between_paths(from_path: &str, to_path: &str) -> String {
        let directory: Vec<&str> = Self::segments(from_path);
        let directory = &directory[..directory.len().saturating_sub(1)];
        let destination_segments = Self::segments(to_path);
        let common_prefix_len = directory
            .iter()
            .zip(&destination_segments)
            .take_while(|(a, b)| a == b)
            .count();
        let mut parts: Vec<&str> = vec![".."; directory.len() - common_prefix_len];

        parts.extend(&destination_segments[common_prefix_len..]);

        parts.join("/")
    }

    fn segments(path: &str) -> Vec<&str> {
        path.split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/imports.rs"]
mod tests;

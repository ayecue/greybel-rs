//! `import_code` expansion used by Grey Hack builds (`PlayerUtils.ImportCode`).

use greybel_source_map::SourceMap;

const TAG: &str = "import_code(";

/// Missing `)` after `import_code(`, which causes an unhandled build exception in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnclosedImport;

/// Expanded code exceeded the configured size limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportedCodeTooLarge;

/// Source file substituted for its import calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub path: String,
    pub code: String,
    /// Source locations for the lines in [`Self::code`].
    pub source_map: SourceMap,
}

/// Returns import paths in source order. Each path is the unquoted text before the next `)` after
/// `import_code(`, unless the call directly follows `//`.
///
/// # Errors
///
/// Returns [`UnclosedImport`] when `import_code(` has no following `)`.
pub fn import_paths(code: &str) -> Result<Vec<String>, UnclosedImport> {
    code.match_indices(TAG)
        .filter(|(start, _)| !code[..*start].ends_with("//"))
        .map(|(start, _)| {
            let after = start + TAG.len();
            let end = code[after..].find(')').ok_or(UnclosedImport)?;

            Ok(code[after..after + end].replace('"', ""))
        })
        .collect()
}

/// Replaces every import call in `code` with its file contents and updates `source_map` accordingly.
/// Imported lines replace the source-map entry for the call's line in `code`.
///
/// # Errors
///
/// Returns [`ImportedCodeTooLarge`] if an import makes the UTF-16 code length exceed `limit`.
pub fn put_imports(
    mut code: String,
    mut source_map: SourceMap,
    imports: &[Import],
    limit: Option<usize>,
) -> Result<(String, SourceMap), ImportedCodeTooLarge> {
    for import in imports {
        let call = format!("{TAG}\"{}\")", import.path);
        let lines = u32::try_from(import.code.matches('\n').count() + 1).unwrap_or(u32::MAX);
        let mut from = 0;

        while let Some(at) = code[from..].find(&call).map(|at| from + at) {
            let line = u32::try_from(code[..at].matches('\n').count() + 1).unwrap_or(u32::MAX);

            code.replace_range(at..at + call.len(), &import.code);
            source_map.splice(line, &import.source_map, lines);
            from = at + import.code.len();
        }

        if limit.is_some_and(|limit| code.encode_utf16().count() > limit) {
            return Err(ImportedCodeTooLarge);
        }
    }

    Ok((code, source_map))
}

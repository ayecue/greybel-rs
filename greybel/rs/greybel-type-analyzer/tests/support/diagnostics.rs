//! Diagnostic fixtures and filters for the document at [`PATH`].

use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Analyzer, Database, DiagnosticKind, Severity, Workspace};

use super::{PATH, uri};

/// Returns `(kind, line, message)` for diagnostics matching `kinds`, with 1-based lines.
pub fn diagnostics(
    workspace: &Workspace<Database>,
    kinds: &[DiagnosticKind],
) -> Vec<(DiagnosticKind, u32, String)> {
    Analyzer::new(workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .filter(|diagnostic| kinds.contains(&diagnostic.kind))
        .map(|diagnostic| {
            (
                diagnostic.kind,
                diagnostic.range.start.line,
                diagnostic.message,
            )
        })
        .collect()
}

/// Returns `(line, message)` for diagnostics of `kind`, with 1-based lines.
pub fn messages_of_kind(
    workspace: &Workspace<Database>,
    kind: DiagnosticKind,
) -> Vec<(u32, String)> {
    diagnostics(workspace, &[kind])
        .into_iter()
        .map(|(_, line, message)| (line, message))
        .collect()
}

pub fn owned(expected: &[(u32, &str)]) -> Vec<(u32, String)> {
    expected
        .iter()
        .map(|&(line, message)| (line, message.to_owned()))
        .collect()
}

/// Returns `(kind, severity)` for every diagnostic.
pub fn severities(workspace: &Workspace<Database>) -> Vec<(DiagnosticKind, Severity)> {
    Analyzer::new(workspace)
        .diagnostics(&uri(PATH))
        .into_iter()
        .map(|diagnostic| (diagnostic.kind, diagnostic.severity))
        .collect()
}

/// Main includes a missing file and imports code from another file.
pub fn missing_files() -> MemoryProvider {
    MemoryProvider::from_iter([(
        PATH,
        "#include \"lib.src\"\nimport_code(\"/other.src\")\nx = 1",
    )])
}

/// Function containing an unused parameter and an unused local.
pub const UNUSED: &str = "f = function(unused)\n  local = 1\nend function";

pub const TEMPLATE_BOUNDS: &str = r#"// @template T extends string
        // @param {T} value
        // @returns {T}
        echo = function(value)
          return value
        end function
        a = echo("x")
        b = echo(1)
        // @template T extends list
        // @param {T} items
        first = function(items)
          return items[0]
        end function
        c = first([1])
        d = first({})"#;

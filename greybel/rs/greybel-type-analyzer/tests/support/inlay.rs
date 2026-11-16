use greybel_core::{Position, Range};
use greybel_type_analyzer::{Analyzer, Dialect, InlayHintKind, InlayHintOptions};

use super::{PATH, uri, with_document};

/// `(line, character, label, kind)` of each inlay hint of `source`.
pub fn hints(source: &str, options: InlayHintOptions) -> Vec<(u32, u32, String, InlayHintKind)> {
    let workspace = with_document(Dialect::Greybel, source);
    let range = Range::new(Position::new(1, 1), Position::new(u32::MAX, 1));

    Analyzer::new(&workspace)
        .inlay_hints(&uri(PATH), range, options)
        .into_iter()
        .map(|hint| {
            (
                hint.position.line,
                hint.position.character,
                hint.label,
                hint.kind,
            )
        })
        .collect()
}

/// Options with only `variable_types`, `parameter_names` or `return_types` turned on.
pub fn only(variable_types: bool, parameter_names: bool, return_types: bool) -> InlayHintOptions {
    InlayHintOptions {
        variable_types,
        parameter_names,
        return_types,
    }
}

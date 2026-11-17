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

#[derive(Debug, Clone, Copy)]
pub enum HintOption {
    VariableTypes,
    ParameterNames,
    ParameterTypes,
    ReturnTypes,
}

/// Options with only `option` turned on.
pub fn only(option: HintOption) -> InlayHintOptions {
    InlayHintOptions {
        variable_types: matches!(option, HintOption::VariableTypes),
        parameter_names: matches!(option, HintOption::ParameterNames),
        parameter_types: matches!(option, HintOption::ParameterTypes),
        return_types: matches!(option, HintOption::ReturnTypes),
    }
}

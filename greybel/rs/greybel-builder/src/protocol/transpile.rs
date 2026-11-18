use greybel_core::{Dialect, SyntaxFeatures};
use greybel_transpiler::{
    BeautifyOptions, BuildMode, Indentation, MultilineStringStyle as TranspiledStyle,
    TranspileOptions, UglifyOptions,
};

use super::{MultilineStringStyle, TranspileMode, TranspileSettings};

impl From<&TranspileSettings> for TranspileOptions {
    fn from(settings: &TranspileSettings) -> Self {
        let mode = match &settings.mode {
            TranspileMode::Default => BuildMode::Default,
            TranspileMode::Uglify {
                disable_literals_optimization,
                disable_namespaces_optimization,
            } => BuildMode::Uglify(UglifyOptions {
                disable_literals_optimization: *disable_literals_optimization,
                disable_namespaces_optimization: *disable_namespaces_optimization,
            }),
            TranspileMode::Beautify {
                retain_parentheses,
                indentation_spaces,
            } => BuildMode::Beautify(BeautifyOptions {
                retain_parentheses: *retain_parentheses,
                indentation: indentation_spaces.map_or(Indentation::Tab, |spaces| {
                    Indentation::Spaces(spaces as usize)
                }),
            }),
        };

        Self {
            dialect: Dialect::GreyScript,
            mode,
            obfuscation: settings.obfuscation,
            environment: settings
                .variables
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            strict_mode: settings.strict_mode,
            syntax_features: SyntaxFeatures::except(
                settings.disabled_syntax.iter().flatten().map(AsRef::as_ref),
            ),
            excluded_names: settings
                .excluded_names
                .iter()
                .map(ToString::to_string)
                .collect(),
            multiline_strings: match settings.multiline_strings {
                Some(MultilineStringStyle::Join) => TranspiledStyle::Join,
                Some(MultilineStringStyle::Concatenation) => TranspiledStyle::Concatenation,
                Some(MultilineStringStyle::Replace) | None => TranspiledStyle::Replace,
            },
            ..TranspileOptions::default()
        }
    }
}

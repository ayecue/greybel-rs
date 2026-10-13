use greybel_core::Dialect;
use greybel_transpiler::{
    BeautifyOptions, BuildMode, Indentation, TranspileOptions, UglifyOptions,
};

use super::{TranspileMode, TranspileSettings};

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
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
            strict_mode: settings.strict_mode,
            excluded_names: settings
                .excluded_names
                .iter()
                .map(ToString::to_string)
                .collect(),
            ..TranspileOptions::default()
        }
    }
}

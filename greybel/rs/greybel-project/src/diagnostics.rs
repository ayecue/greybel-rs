use std::borrow::Cow;
use std::collections::BTreeMap;

use greybel_type_analyzer::{DiagnosticKind, DiagnosticOptions, Severity};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

/// What a project makes of one kind of diagnostic: `off` drops it, a severity replaces its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel {
    Off,
    Error,
    Warning,
    Information,
    Hint,
}

/// Diagnostic levels by diagnostic code, e.g. `{"used-before-defined": "off", "shadowing": "error"}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagnosticSettings(pub BTreeMap<String, DiagnosticLevel>);

impl DiagnosticSettings {
    /// The type analyzer's options: outside strict mode `call-signature-mismatch` is off unless set; codes the
    /// analyzer doesn't know are skipped.
    #[must_use]
    pub fn options(&self, strict_mode: bool) -> DiagnosticOptions {
        let mut options = DiagnosticOptions::default();

        if !strict_mode {
            options
                .disabled_kinds
                .insert(DiagnosticKind::CallSignatureMismatch);
        }

        for (code, &level) in &self.0 {
            let Some(kind) = DiagnosticKind::from_code(code) else {
                continue;
            };
            let severity = match level {
                DiagnosticLevel::Off => {
                    options.disabled_kinds.insert(kind);
                    continue;
                }
                DiagnosticLevel::Error => Severity::Error,
                DiagnosticLevel::Warning => Severity::Warning,
                DiagnosticLevel::Information => Severity::Information,
                DiagnosticLevel::Hint => Severity::Hint,
            };

            options.disabled_kinds.remove(&kind);
            options.severity_overrides.insert(kind, severity);
        }

        options
    }
}

impl JsonSchema for DiagnosticSettings {
    fn schema_name() -> Cow<'static, str> {
        "DiagnosticSettings".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let codes: Vec<&str> = DiagnosticKind::ALL.iter().map(|kind| kind.code()).collect();
        let level = generator.subschema_for::<DiagnosticLevel>();

        json_schema!({
            "type": "object",
            "propertyNames": { "enum": codes },
            "additionalProperties": level,
        })
    }
}

impl crate::Manifest {
    /// The type analyzer's options for this project's `diagnostics` and `strictMode`.
    #[must_use]
    pub fn diagnostic_options(&self) -> DiagnosticOptions {
        self.diagnostics.options(self.strict_mode)
    }
}

use std::collections::{HashMap, HashSet};

use greybel_core::{ColumnSettings, LineIndex, ParseError, Range, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub range: Range,
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(
        kind: DiagnosticKind,
        lines: &LineIndex,
        span: Span,
        message: String,
    ) -> Self {
        Self {
            kind,
            severity: kind.default_severity(),
            range: lines.range(span, ColumnSettings::default()),
            message,
        }
    }

    pub(crate) fn syntax(error: &ParseError, lines: &LineIndex) -> Self {
        Self::new(DiagnosticKind::Syntax, lines, error.span, error.to_string())
    }

    pub(crate) fn sort(diagnostics: &mut [Self]) {
        diagnostics.sort_by_key(|diagnostic| diagnostic.range.start);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    Syntax,
    UnresolvedReference,
    InvalidAnnotation,
    CallArityMismatch,
    MissingImport,
    CyclicImport,
    DeadCodeAfterTerminator,
    DuplicateKey,
    CallOfNonCallable,
    IndexOfNonIndexable,
    CallSignatureMismatch,
    TypeMismatch,
    AnnotationSignatureMismatch,
    UnusedLocal,
    UnusedParameter,
    Shadow,
    SelfAssignment,
    ConstantCondition,
    UseBeforeDefinition,
    CyclicInheritance,
    CyclicTypeDefinition,
    IgnoredImportCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticKind {
    #[must_use]
    pub fn default_severity(self) -> Severity {
        match self {
            Self::Syntax
            | Self::UnresolvedReference
            | Self::MissingImport
            | Self::CyclicImport
            | Self::DuplicateKey
            | Self::CallOfNonCallable
            | Self::IndexOfNonIndexable
            | Self::TypeMismatch
            | Self::AnnotationSignatureMismatch
            | Self::CyclicInheritance
            | Self::CyclicTypeDefinition => Severity::Error,
            Self::InvalidAnnotation
            | Self::CallArityMismatch
            | Self::DeadCodeAfterTerminator
            | Self::CallSignatureMismatch
            | Self::UnusedLocal
            | Self::UnusedParameter
            | Self::Shadow
            | Self::SelfAssignment
            | Self::ConstantCondition
            | Self::UseBeforeDefinition
            | Self::IgnoredImportCode => Severity::Warning,
        }
    }

    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Syntax => "syntax-error",
            Self::UnresolvedReference => "unresolved-reference",
            Self::InvalidAnnotation => "invalid-annotation",
            Self::CallArityMismatch => "call-arity-mismatch",
            Self::MissingImport => "missing-import",
            Self::CyclicImport => "cyclic-import",
            Self::DeadCodeAfterTerminator => "dead-code-after-terminator",
            Self::DuplicateKey => "map-literal-duplicate-keys",
            Self::CallOfNonCallable => "call-of-non-callable",
            Self::IndexOfNonIndexable => "index-of-non-indexable",
            Self::CallSignatureMismatch => "call-signature-mismatch",
            Self::TypeMismatch => "type-mismatch",
            Self::AnnotationSignatureMismatch => "annotation-signature-mismatch",
            Self::UnusedLocal => "unused-local",
            Self::UnusedParameter => "unused-parameter",
            Self::Shadow => "shadowing",
            Self::SelfAssignment => "self-assignment",
            Self::ConstantCondition => "constant-condition",
            Self::UseBeforeDefinition => "used-before-defined",
            Self::CyclicInheritance => "cyclic-inheritance",
            Self::CyclicTypeDefinition => "cyclic-type-definition",
            Self::IgnoredImportCode => "ignored-import-code",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticOptions {
    pub severity_overrides: HashMap<DiagnosticKind, Severity>,
    pub disabled_kinds: HashSet<DiagnosticKind>,
}

impl DiagnosticOptions {
    #[must_use]
    pub fn apply(&self, diagnostics: impl IntoIterator<Item = Diagnostic>) -> Vec<Diagnostic> {
        diagnostics
            .into_iter()
            .filter(|diagnostic| !self.disabled_kinds.contains(&diagnostic.kind))
            .map(|mut diagnostic| {
                if let Some(&severity) = self.severity_overrides.get(&diagnostic.kind) {
                    diagnostic.severity = severity;
                }

                diagnostic
            })
            .collect()
    }
}

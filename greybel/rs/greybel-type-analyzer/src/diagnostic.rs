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

macro_rules! diagnostic_kinds {
    (@enabled off) => {
        false
    };
    (@enabled) => {
        true
    };
    ($($kind:ident => $code:literal, $severity:ident $(, $off:ident)?;)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum DiagnosticKind {
            $($kind,)*
        }

        impl DiagnosticKind {
            pub const ALL: &'static [Self] = &[$(Self::$kind,)*];

            #[must_use]
            pub fn code(self) -> &'static str {
                match self {
                    $(Self::$kind => $code,)*
                }
            }

            #[must_use]
            pub fn default_severity(self) -> Severity {
                match self {
                    $(Self::$kind => Severity::$severity,)*
                }
            }

            /// Whether the diagnostic is reported without the options naming it; `off` kinds need a severity in
            /// [`DiagnosticOptions::severity_overrides`].
            #[must_use]
            pub fn is_enabled_by_default(self) -> bool {
                match self {
                    $(Self::$kind => diagnostic_kinds!(@enabled $($off)?),)*
                }
            }
        }
    };
}

diagnostic_kinds! {
    Syntax => "syntax-error", Error;
    UnresolvedReference => "unresolved-reference", Error;
    InvalidAnnotation => "invalid-annotation", Warning;
    CallArityMismatch => "call-arity-mismatch", Warning;
    MissingImport => "missing-import", Error;
    CyclicImport => "cyclic-import", Error;
    DeadCodeAfterTerminator => "dead-code-after-terminator", Warning;
    DuplicateKey => "map-literal-duplicate-keys", Error;
    CallOfNonCallable => "call-of-non-callable", Error;
    IndexOfNonIndexable => "index-of-non-indexable", Error;
    CallSignatureMismatch => "call-signature-mismatch", Warning;
    TypeMismatch => "type-mismatch", Error;
    AnnotationSignatureMismatch => "annotation-signature-mismatch", Error;
    UnusedLocal => "unused-local", Warning;
    UnusedParameter => "unused-parameter", Warning;
    Shadow => "shadowing", Warning;
    SelfAssignment => "self-assignment", Warning;
    ConstantCondition => "constant-condition", Warning;
    UseBeforeDefinition => "used-before-defined", Warning;
    CyclicInheritance => "cyclic-inheritance", Error;
    CyclicTypeDefinition => "cyclic-type-definition", Error;
    IgnoredImportCode => "ignored-import-code", Warning;
    MisplacedImportCode => "misplaced-import-code", Warning;
    NativeOverride => "native-override", Warning;
    UnknownMember => "unknown-member", Warning;
    PrivateMember => "private-member", Warning;
    MissingImplementation => "missing-implementation", Warning;
    MissingCase => "missing-case", Warning;
    ImpossibleCase => "impossible-case", Warning;
    OptionalMember => "optional-member", Warning;
    ReadonlyMember => "readonly-member", Warning;
    InvalidOperation => "invalid-operation", Warning;
    Deprecated => "deprecated", Hint;
    ConstAssignment => "const-assignment", Error;
    DuplicateDeclaration => "duplicate-declaration", Error;
    MissingLet => "missing-let", Warning, off;
    ImplicitAny => "implicit-any", Warning, off;
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
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.code() == code)
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
            .filter(|diagnostic| {
                let kind = diagnostic.kind;

                !self.disabled_kinds.contains(&kind)
                    && (kind.is_enabled_by_default() || self.severity_overrides.contains_key(&kind))
            })
            .map(|mut diagnostic| {
                if let Some(&severity) = self.severity_overrides.get(&diagnostic.kind) {
                    diagnostic.severity = severity;
                }

                diagnostic
            })
            .collect()
    }
}

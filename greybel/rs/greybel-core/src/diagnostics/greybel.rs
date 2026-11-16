use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GreybelDiagnostic {
    #[error("expected 'from'")]
    MissingFromKeyword,
    #[error("'{keyword}' needs an assignment to a variable")]
    DeclarationWithoutAssignment { keyword: &'static str },
    #[error("'enum' needs a name")]
    EnumName,
    #[error("expected an enum entry name")]
    EnumEntryName,
    #[error("an enum value must be a number or string literal")]
    EnumValue,
    #[error("an enum entry after a string value needs its own value")]
    MissingEnumValue,
    #[error("the enum already has an entry '{name}'")]
    DuplicateEnumEntry { name: String },
    #[error("'enum' without matching 'end enum'")]
    UnclosedEnum,
    #[error("a ternary expression needs ':' followed by the value for a false condition")]
    MissingTernaryElse,
    #[error("cannot assign to an optional chain")]
    OptionalChainAssignment,
    #[error("'export' is only allowed at the top level of a file")]
    NestedExport,
    #[error("'export' needs an assignment to a variable")]
    ExportWithoutAssignment,
    #[error("a switch can have only one 'else'")]
    SecondSwitchElse,
    #[error("statements in a switch must be inside a 'case'")]
    StatementOutsideCase,
    #[error("expected a type")]
    ExpectedType,
    #[error(
        "a type body can only contain members (name: type) and methods (name(parameters): type)"
    )]
    TypeMember,
    #[error("'type' without matching 'end type'")]
    UnclosedType,
    #[error("'class' needs a name")]
    ClassName,
    #[error("'implements' needs type names")]
    ImplementsName,
    #[error("a class body can only contain fields (name = value) and methods (name(parameters))")]
    ClassMember,
    #[error("the class already has a member '{name}'")]
    DuplicateClassMember { name: String },
    #[error(
        "a class that extends another needs a constructor when it has instance fields; write static before a field \
         all instances share"
    )]
    InstanceFieldsWithoutConstructor,
    #[error("a template placeholder needs an expression")]
    EmptyTemplatePlaceholder,
    #[error("expected '}}' after the expression of a template placeholder")]
    UnclosedTemplatePlaceholder,
}

impl GreybelDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::MissingFromKeyword
            | Self::ExpectedType
            | Self::UnclosedTemplatePlaceholder
            | Self::MissingTernaryElse => ErrorCode::ExpectedToken,
            Self::DeclarationWithoutAssignment { .. }
            | Self::ExportWithoutAssignment
            | Self::OptionalChainAssignment => ErrorCode::InvalidAssignmentTarget,
            Self::EmptyTemplatePlaceholder | Self::EnumValue | Self::MissingEnumValue => {
                ErrorCode::ExpectedExpression
            }
            Self::EnumName
            | Self::EnumEntryName
            | Self::DuplicateEnumEntry { .. }
            | Self::SecondSwitchElse
            | Self::StatementOutsideCase
            | Self::NestedExport
            | Self::ClassName
            | Self::TypeMember
            | Self::ImplementsName
            | Self::ClassMember
            | Self::DuplicateClassMember { .. }
            | Self::InstanceFieldsWithoutConstructor => ErrorCode::UnexpectedToken,
            Self::UnclosedEnum | Self::UnclosedType => ErrorCode::UnclosedBlock,
        }
    }
}

use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ExpectationDiagnostic {
    #[error("got {got} where {expectation} is required")]
    Token { got: String, expectation: String },
    #[error("got {got} where {} is required", one_of(.expectations))]
    OneOf {
        got: String,
        expectations: Vec<String>,
    },
    #[error("got {got} where number, string, or identifier is required")]
    Expression { got: String },
    #[error("got {got} where Identifier is required")]
    Identifier { got: String },
    #[error("got {got} where an identifier is required")]
    Parameter { got: String },
}

fn one_of(expectations: &[String]) -> String {
    let mut distinct: Vec<String> = Vec::with_capacity(expectations.len());

    for expectation in expectations {
        if !distinct.contains(expectation) {
            distinct.push(expectation.clone());
        }
    }

    match distinct.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {last}", init.join(", ")),
    }
}

impl ExpectationDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Token { .. }
            | Self::OneOf { .. }
            | Self::Identifier { .. }
            | Self::Parameter { .. } => ErrorCode::ExpectedToken,
            Self::Expression { .. } => ErrorCode::ExpectedExpression,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        None
    }
}

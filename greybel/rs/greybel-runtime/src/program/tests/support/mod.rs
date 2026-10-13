use futures_util::FutureExt;

use crate::program::{EvaluatedText, Evaluator};
use crate::protocol::EvaluationFailure;

/// Test evaluator returning 42 for `x`, `"n""eo"` for `name`, and an undefined-identifier failure otherwise.
pub(super) struct Values;

impl Evaluator for Values {
    async fn evaluate_text(
        &mut self,
        code: &str,
    ) -> Result<Option<EvaluatedText>, EvaluationFailure> {
        match code {
            "x" => Ok(Some(EvaluatedText {
                text: "42".to_owned(),
                type_name: "number".to_owned(),
            })),
            "name" => Ok(Some(EvaluatedText {
                text: "\"n\"\"eo\"".to_owned(),
                type_name: "string".to_owned(),
            })),
            _ => Err(EvaluationFailure::from_message(format!(
                "Undefined Identifier: '{code}'"
            ))),
        }
    }
}

pub(super) fn interpolated(message: &str) -> String {
    Values
        .interpolate(message)
        .now_or_never()
        .expect("evaluates at once")
}

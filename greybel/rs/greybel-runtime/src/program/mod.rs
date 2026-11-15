//! Program implementations for editor launches in simulated and game environments.

mod binary;
#[cfg(feature = "game")]
mod game;
mod mock;

#[cfg(feature = "game")]
pub(crate) use game::GameProgram;
pub(crate) use mock::MockProgram;

use crate::protocol::EvaluationFailure;

/// Rendered expression value with its type name.
pub(crate) struct EvaluatedText {
    pub text: String,
    pub type_name: String,
}

impl EvaluatedText {
    /// Removes surrounding string quotes and unescapes doubled quotes; omits changes to other rendered values.
    fn log_text(&self) -> String {
        if self.type_name != "string" {
            return self.text.clone();
        }

        self.text
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .map_or_else(|| self.text.clone(), |text| text.replace("\"\"", "\""))
    }
}

/// Evaluates breakpoint conditions and log-message expressions in stopped programs.
pub(crate) trait Evaluator {
    /// Evaluates `code` to a rendered expression value, or `None` for statements.
    async fn evaluate_text(
        &mut self,
        code: &str,
    ) -> Result<Option<EvaluatedText>, EvaluationFailure>;

    /// Expands `{expression}` placeholders in `message`. Strings are unquoted, other values render normally, errors
    /// become `{error: …}`, and doubled braces emit literal braces.
    async fn interpolate(&mut self, message: &str) -> String {
        let mut text = String::new();
        let mut remainder = message;

        while let Some(start) = remainder.find(['{', '}']) {
            text.push_str(&remainder[..start]);

            let brace = &remainder[start..];

            if brace.starts_with("{{") || brace.starts_with("}}") {
                text.push_str(&brace[..1]);
                remainder = &brace[2..];
                continue;
            }

            let Some(end) = brace.find('}').filter(|_| brace.starts_with('{')) else {
                text.push_str(&brace[..1]);
                remainder = &brace[1..];
                continue;
            };
            let expression = &brace[1..end];

            match self.evaluate_text(expression).await {
                Ok(Some(value)) => text.push_str(&value.log_text()),
                Ok(None) => text.push_str("null"),
                Err(failure) => text.push_str(&format!("{{error: {failure}}}")),
            }

            remainder = &brace[end + 1..];
        }

        text.push_str(remainder);

        text
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

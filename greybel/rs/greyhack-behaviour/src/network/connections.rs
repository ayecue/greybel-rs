//! Parameter validation for game commands.

/// Tokens that `OS.IsParamsAllowed` rejects in parameters.
pub const FORBIDDEN_PARAM_TOKENS: &[&str] = &["[", "]", "//", ";", "\"", "{", "}", "(", ")", "\\"];

/// Finds the first token in `param` rejected by `OS.IsParamsAllowed`.
#[must_use]
pub fn forbidden_param_token(param: &str) -> Option<&'static str> {
    FORBIDDEN_PARAM_TOKENS
        .iter()
        .copied()
        .find(|forbidden| param.contains(forbidden))
}

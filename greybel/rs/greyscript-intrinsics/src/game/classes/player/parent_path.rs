use greybel_interpreter::{Value, native};
use greyhack_behaviour::path;

use crate::error::IntrinsicError;

#[native]
pub fn parent_path(path: &Value) -> Result<String, IntrinsicError> {
    match path.as_str() {
        Some(path) if !path.is_empty() => Ok(path::parent(path).to_owned()),
        _ => Err(IntrinsicError::InvalidArguments("parent_path")),
    }
}
